use std::{
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::*,
    Storage::FileSystem::{ReadFile, WriteFile},
    System::{IO::*, Threading::CreateEventW},
};

pub struct Overlap {
    data: Box<OVERLAPPED>,
    _event: OwnedHandle,
}

// The operation owns its stable allocation and is never shared across threads.
unsafe impl Send for Overlap {}

impl Overlap {
    pub fn new() -> io::Result<Self> {
        let event = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
        if event.is_null() {
            return Err(io::Error::last_os_error());
        }
        let event = unsafe { OwnedHandle::from_raw_handle(event) };
        let mut data: Box<OVERLAPPED> = Box::new(unsafe { std::mem::zeroed() });
        data.hEvent = event.as_raw_handle();
        Ok(Self {
            data,
            _event: event,
        })
    }

    pub fn pointer(&mut self) -> *mut OVERLAPPED {
        &mut *self.data
    }

    pub fn ready(&self, handle: &OwnedHandle) -> io::Result<bool> {
        let mut transferred = 0;
        let result = unsafe {
            GetOverlappedResult(handle.as_raw_handle(), &*self.data, &mut transferred, 0)
        };
        if result != 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_IO_INCOMPLETE as i32) {
            return Ok(false);
        }
        Err(error)
    }

    pub fn cancel(&self, handle: &OwnedHandle) {
        unsafe {
            CancelIoEx(handle.as_raw_handle(), &*self.data);
            let mut transferred = 0;
            GetOverlappedResult(handle.as_raw_handle(), &*self.data, &mut transferred, 1);
        }
    }

    fn finish(&self, handle: &OwnedHandle, deadline: Instant) -> io::Result<usize> {
        let wait = match deadline.checked_duration_since(Instant::now()) {
            Some(wait) if !wait.is_zero() => {
                wait.as_millis().saturating_add(1).min(u32::MAX as u128 - 1) as u32
            }
            _ => {
                self.cancel(handle);
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "cli: pipe deadline expired; operation outcome may be unknown",
                ));
            }
        };
        let mut transferred = 0;
        let result = unsafe {
            GetOverlappedResultEx(
                handle.as_raw_handle(),
                &*self.data,
                &mut transferred,
                wait,
                0,
            )
        };
        if result != 0 {
            return Ok(transferred as usize);
        }
        let error = io::Error::last_os_error();
        self.cancel(handle);
        if matches!(error.raw_os_error(), Some(code) if code == WAIT_TIMEOUT as i32 || code == ERROR_SEM_TIMEOUT as i32)
        {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "cli: pipe response timed out; operation outcome may be unknown",
            ));
        }
        Err(error)
    }
}

pub fn read(handle: &OwnedHandle, buffer: &mut [u8], deadline: Instant) -> io::Result<usize> {
    transfer(handle, buffer.as_mut_ptr(), buffer.len(), deadline, false)
}

pub fn write(handle: &OwnedHandle, buffer: &[u8], deadline: Instant) -> io::Result<usize> {
    transfer(
        handle,
        buffer.as_ptr().cast_mut(),
        buffer.len(),
        deadline,
        true,
    )
}

fn transfer(
    handle: &OwnedHandle,
    buffer: *mut u8,
    length: usize,
    deadline: Instant,
    write: bool,
) -> io::Result<usize> {
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "cli: pipe deadline expired",
        ));
    }
    let mut operation = Overlap::new()?;
    let mut transferred = 0;
    let result = unsafe {
        if write {
            WriteFile(
                handle.as_raw_handle(),
                buffer,
                length.min(u32::MAX as usize) as u32,
                &mut transferred,
                operation.pointer(),
            )
        } else {
            ReadFile(
                handle.as_raw_handle(),
                buffer,
                length.min(u32::MAX as usize) as u32,
                &mut transferred,
                operation.pointer(),
            )
        }
    };
    if result != 0 {
        return Ok(transferred as usize);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
        return Err(error);
    }
    operation.finish(handle, deadline)
}
