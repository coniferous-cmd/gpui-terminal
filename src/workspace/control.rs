use gpui::{Context, Window};

use super::Workspace;
use crate::cli::protocol::{ErrorCode, Operation, Response, SessionInfo};

impl Workspace {
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    pub fn session_info(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .enumerate()
            .map(|(index, session)| SessionInfo {
                id: session.runtime_id.clone(),
                window_id: self.runtime_id.clone(),
                name: session.named().to_string(),
                cwd: session.current().folder.to_string_lossy().into_owned(),
                tab_count: session.tabs.len(),
                selected: self.active == index,
            })
            .collect()
    }

    pub fn control(
        &mut self,
        operation: Operation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Response {
        let id = match &operation {
            Operation::List {} => {
                return Response::Listed {
                    sessions: self.session_info(),
                };
            }
            Operation::Create {
                window: target,
                name,
                cwd,
            } => {
                if target != &self.runtime_id {
                    return not_found();
                }
                let result = cwd
                    .as_ref()
                    .map(|path| path.to_path())
                    .transpose()
                    .and_then(|folder| {
                        self.create_terminal(
                            true,
                            folder,
                            name.clone().unwrap_or_default(),
                            window,
                            cx,
                        )
                    });
                return match result {
                    Ok(()) => Response::Created {
                        id: self.sessions[self.active].runtime_id.clone(),
                    },
                    Err(error) => Response::Error {
                        code: ErrorCode::OperationFailed,
                        message: format!("cli: creating session: {error:#}"),
                    },
                };
            }
            Operation::Rename { id, .. }
            | Operation::Focus { id }
            | Operation::Close { id, .. } => id,
        };
        let Some(index) = self
            .sessions
            .iter()
            .position(|session| &session.runtime_id == id)
        else {
            return not_found();
        };
        match operation {
            Operation::Rename { name, .. } => {
                let id = self.sessions[index].id;
                self.end_rename(id, false, window, cx);
                Self::set_session_name(&mut self.sessions[index], &name);
                self.retitle(window);
                self.note(true, cx);
                cx.notify();
            }
            Operation::Focus { .. } => {
                self.select(index, window, cx);
                window.activate_window();
            }
            Operation::Close { force, .. } => {
                if self.sessions[index].tabs.len() > 1 && !force {
                    return Response::Error {
                        code: ErrorCode::ConfirmationRequired,
                        message: "cli: session contains multiple tabs; use --force to close all tabs and their programs".into(),
                    };
                }
                self.close(index, window, cx);
            }
            _ => unreachable!("list and create returned before session lookup"),
        }
        Response::Applied {}
    }
}

fn not_found() -> Response {
    Response::Error {
        code: ErrorCode::NotFound,
        message: "cli: session or window no longer exists".into(),
    }
}
