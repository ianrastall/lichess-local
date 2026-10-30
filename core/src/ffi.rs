use crate::{
    game::Service,
    protocol::{Config, Request, Response},
};
use prost::Message;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    ptr,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
pub struct Core {
    service: Mutex<Service>,
    cancelled: AtomicBool,
}
pub struct ResultBuffer(Vec<u8>);
fn buffer(response: Response) -> *mut ResultBuffer {
    Box::into_raw(Box::new(ResultBuffer(response.encode_to_vec())))
}
#[unsafe(no_mangle)]
pub extern "C" fn ll_abi_version() -> u32 {
    1
}

/// Input bytes remain owned by the caller. Out pointers are initialized before decoding.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ll_create(
    bytes: *const u8,
    length: usize,
    core: *mut *mut Core,
    result: *mut *mut ResultBuffer,
) -> u32 {
    if core.is_null() || result.is_null() {
        return 1;
    }
    unsafe {
        *core = ptr::null_mut();
        *result = ptr::null_mut();
    }
    if bytes.is_null() || length > 1_048_576 {
        return 1;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let config = Config::decode(unsafe { std::slice::from_raw_parts(bytes, length) })
            .map_err(|e| e.to_string())?;
        let service = Service::open(Path::new(&config.directory), Path::new(&config.engine))?;
        Ok::<_, String>(Core {
            service: Mutex::new(service),
            cancelled: AtomicBool::new(false),
        })
    })) {
        Ok(Ok(value)) => {
            unsafe {
                *core = Box::into_raw(Box::new(value));
            }
            0
        }
        Ok(Err(error)) => {
            unsafe {
                *result = buffer(Response {
                    error,
                    ..Default::default()
                });
            }
            2
        }
        Err(_) => 3,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ll_call(
    core: *mut Core,
    bytes: *const u8,
    length: usize,
    result: *mut *mut ResultBuffer,
) -> u32 {
    if result.is_null() {
        return 1;
    }
    unsafe {
        *result = ptr::null_mut();
    }
    if core.is_null() || bytes.is_null() || length > 1_048_576 {
        return 1;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let request = Request::decode(unsafe { std::slice::from_raw_parts(bytes, length) })
            .map_err(|_| 2u32)?;
        let core = unsafe { &*core };
        let mut service = core.service.lock().map_err(|_| 4u32)?;
        core.cancelled.store(false, Ordering::Release);
        let response = match service.handle(&request, &core.cancelled) {
            Ok(snapshot) => Response {
                request_id: request.request_id,
                ok: true,
                snapshot: Some(snapshot),
                error: String::new(),
            },
            Err(error) => Response {
                request_id: request.request_id,
                ok: false,
                snapshot: service.snapshot().ok(),
                error,
            },
        };
        unsafe {
            *result = buffer(response);
        }
        Ok::<_, u32>(())
    })) {
        Ok(Ok(())) => 0,
        Ok(Err(status)) => status,
        Err(_) => 3,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ll_cancel(core: *mut Core) {
    if !core.is_null() {
        unsafe { &*core }.cancelled.store(true, Ordering::Release);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ll_result_bytes(
    result: *const ResultBuffer,
    bytes: *mut *const u8,
    length: *mut usize,
) -> u32 {
    if bytes.is_null() || length.is_null() {
        return 1;
    }
    unsafe {
        *bytes = ptr::null();
        *length = 0;
    }
    if result.is_null() {
        return 1;
    }
    unsafe {
        *bytes = (*result).0.as_ptr();
        *length = (*result).0.len();
    }
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ll_result_free(result: *mut ResultBuffer) {
    if !result.is_null() {
        unsafe {
            drop(Box::from_raw(result));
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ll_destroy(core: *mut Core) {
    if !core.is_null() {
        unsafe {
            drop(Box::from_raw(core));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protobuf_abi_roundtrip_and_null_contract() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            directory: dir.path().to_str().unwrap().into(),
            engine: "missing".into(),
        }
        .encode_to_vec();
        let mut core = ptr::null_mut();
        let mut result = ptr::null_mut();
        unsafe {
            assert_eq!(
                ll_create(config.as_ptr(), config.len(), &mut core, &mut result),
                0
            );
            assert!(!core.is_null());
            assert!(result.is_null());
            let request = Request {
                request_id: 19,
                action: "snapshot".into(),
                ..Default::default()
            }
            .encode_to_vec();
            assert_eq!(
                ll_call(core, request.as_ptr(), request.len(), &mut result),
                0
            );
            let mut bytes = ptr::null();
            let mut length = 0;
            assert_eq!(ll_result_bytes(result, &mut bytes, &mut length), 0);
            let response = Response::decode(std::slice::from_raw_parts(bytes, length)).unwrap();
            assert!(response.ok);
            assert_eq!(response.request_id, 19);
            ll_result_free(result);
            assert_eq!(ll_call(core, ptr::null(), 1, &mut result), 1);
            assert!(result.is_null());
            ll_destroy(core);
        }
    }
}
