use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use futures::{StreamExt, channel::mpsc as channel};
use gpui::{App, Global};

use super::{
    identity, platform,
    protocol::{ErrorCode, Operation, Request, Response, read_frame, write_frame},
};
use crate::workspace::Workspace;

struct Service {
    pending: Arc<AtomicUsize>,
    stopping: Arc<AtomicBool>,
}
impl Global for Service {}

pub fn defer_quit(cx: &mut App) -> bool {
    let Some(service) = cx.try_global::<Service>() else {
        return false;
    };
    if service.pending.load(Ordering::Acquire) == 0 {
        return false;
    }
    service.stopping.store(true, Ordering::Release);
    let pending = service.pending.clone();
    cx.spawn(async move |cx| {
        let until = Instant::now() + Duration::from_secs(11);
        while pending.load(Ordering::Acquire) != 0 && Instant::now() < until {
            cx.background_executor()
                .timer(Duration::from_millis(10))
                .await;
        }
        if pending.load(Ordering::Acquire) != 0 {
            log::error!("cli: timed out waiting for final replies before quitting");
        }
        cx.update(|cx| cx.quit());
    })
    .detach();
    true
}

struct Job {
    request: Request,
    reply: mpsc::SyncSender<Response>,
}

pub fn start(cx: &mut App) -> anyhow::Result<()> {
    let root = platform::registry()?;
    let listener = platform::Listener::bind(&root, identity::instance())?;
    let stopping = Arc::new(AtomicBool::new(false));
    let pending = Arc::new(AtomicUsize::new(0));
    cx.set_global(Service {
        pending: pending.clone(),
        stopping: stopping.clone(),
    });
    let (sender, mut receiver) = channel::channel::<Job>(32);
    let stop = stopping.clone();
    cx.spawn(async move |cx| {
        while let Some(job) = receiver.next().await {
            let response = if stop.load(Ordering::Acquire) {
                failure("cli: application is shutting down")
            } else {
                cx.update(|cx| dispatch(job.request.operation, cx))
            };
            if job.reply.send(response).is_err() {
                log::warn!("cli: requester disconnected after dispatch");
            }
        }
    })
    .detach();
    let stop = stopping.clone();
    let active = pending.clone();
    let ended = Arc::new(AtomicBool::new(false));
    let worker_ended = ended.clone();
    std::thread::Builder::new().name("session-cli".into()).spawn(move || {
        while !stop.load(Ordering::Acquire) {
            match listener.accept() {
                Ok(Some(mut stream)) => {
                    if active.load(Ordering::Acquire) >= 32 {
                        log::warn!("cli: control connection limit reached");
                        continue;
                    }
                    let pending = active.clone();
                    let mut sender = sender.clone();
                    pending.fetch_add(1, Ordering::AcqRel);
                    let counted = Pending(pending);
                    let result = std::thread::Builder::new().name("session-cli-request".into()).spawn(move || {
                        let _counted = counted;
                        let response = match read_frame::<Request>(&mut stream) {
                            Ok(request) => match request.validate() {
                                Ok(()) => {
                                    let (reply, receive) = mpsc::sync_channel(1);
                                    if sender.try_send(Job { request, reply }).is_err() {
                                        failure("cli: GUI command queue is full or closed")
                                    } else {
                                        receive.recv_timeout(platform::RESPONSE_WAIT).unwrap_or_else(|_| failure("cli: response timed out; operation outcome is unknown"))
                                    }
                                }
                                Err(error) => invalid(&format!("{error:#}")),
                            },
                            Err(error) => invalid(&format!("{error:#}")),
                        };
                        if let Err(error) = write_frame(&mut stream, &response) {
                            log::warn!("cli: replying to command: {error:#}");
                        } else {
                            let mut acknowledgment = [0];
                            if let Err(error) = stream.read_exact(&mut acknowledgment) {
                                log::warn!("cli: awaiting response acknowledgment: {error}");
                            } else if acknowledgment != [1] {
                                log::warn!("cli: invalid response acknowledgment");
                            }
                        }
                    });
                    if let Err(error) = result {
                        log::error!("cli: starting request worker: {error}");
                    }
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(error) => {
                    log::error!("cli: accepting command: {error:#}");
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
        drop(listener);
        worker_ended.store(true, Ordering::Release);
    })?;
    cx.on_app_quit(move |cx| {
        stopping.store(true, Ordering::Release);
        let pending = pending.clone();
        let ended = ended.clone();
        let executor = cx.background_executor().clone();
        async move {
            let until = Instant::now() + Duration::from_millis(100);
            while (pending.load(Ordering::Acquire) != 0 || !ended.load(Ordering::Acquire))
                && Instant::now() < until
            {
                executor.timer(Duration::from_millis(10)).await;
            }
            if pending.load(Ordering::Acquire) != 0 {
                log::error!("cli: shutdown timed out waiting for command replies");
            }
        }
    })
    .detach();
    Ok(())
}

struct Pending(Arc<AtomicUsize>);
impl Drop for Pending {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn dispatch(operation: Operation, cx: &mut App) -> Response {
    log::debug!(
        "cli: dispatching {}",
        match &operation {
            Operation::List {} => "list",
            Operation::Create { .. } => "create",
            Operation::Rename { .. } => "rename",
            Operation::Focus { .. } => "focus",
            Operation::Close { .. } => "close",
        }
    );
    let windows: Vec<_> = cx
        .windows()
        .into_iter()
        .filter_map(|handle| handle.downcast::<Workspace>())
        .collect();
    if matches!(operation, Operation::List {}) {
        let mut sessions = Vec::new();
        for handle in windows {
            match handle.update(cx, |workspace, _, _| workspace.session_info()) {
                Ok(listed) => sessions.extend(listed),
                Err(error) => return failure(&format!("cli: listing window: {error}")),
            }
        }
        return Response::Listed { sessions };
    }
    let target = windows.into_iter().find(|handle| {
        handle
            .update(cx, |workspace, _, _| match &operation {
                Operation::Create { window, .. } => workspace.runtime_id() == window,
                Operation::Rename { id, .. }
                | Operation::Focus { id }
                | Operation::Close { id, .. } => workspace
                    .sessions()
                    .iter()
                    .any(|session| &session.runtime_id == id),
                Operation::List {} => false,
            })
            .unwrap_or(false)
    });
    match target {
        Some(handle) => handle
            .update(cx, |workspace, window, cx| {
                workspace.control(operation, window, cx)
            })
            .unwrap_or_else(|error| failure(&format!("cli: applying command: {error}"))),
        None => Response::Error {
            code: ErrorCode::NotFound,
            message: "cli: session or window no longer exists".into(),
        },
    }
}

fn failure(message: &str) -> Response {
    Response::Error {
        code: ErrorCode::OperationFailed,
        message: message.into(),
    }
}

fn invalid(message: &str) -> Response {
    Response::Error {
        code: ErrorCode::InvalidRequest,
        message: message.into(),
    }
}
