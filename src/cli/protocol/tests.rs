use super::*;

fn framed(bytes: &[u8]) -> Vec<u8> {
    let mut result = (bytes.len() as u32).to_be_bytes().to_vec();
    result.extend_from_slice(bytes);
    result
}

#[test]
fn requests_round_trip_with_validation() {
    for operation in [
        Operation::List {},
        Operation::Create {
            window: "instance-w1".into(),
            name: Some("共享".into()),
            cwd: None,
        },
        Operation::Rename {
            id: "instance-s1".into(),
            name: String::new(),
        },
        Operation::Focus {
            id: "instance-s1".into(),
        },
        Operation::Close {
            id: "instance-s1".into(),
            force: true,
        },
    ] {
        let request = Request::new(operation);
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &request).unwrap();
        let decoded: Request = read_frame(&mut bytes.as_slice()).unwrap();
        decoded.validate().unwrap();
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::to_value(decoded).unwrap()
        );
    }
}

#[test]
fn responses_round_trip() {
    for response in [
        Response::Listed {
            sessions: vec![SessionInfo {
                id: "i-s1".into(),
                window_id: "i-w1".into(),
                name: "name\nwith controls".into(),
                cwd: "/tmp".into(),
                tab_count: 2,
                selected: true,
            }],
        },
        Response::Created { id: "i-s2".into() },
        Response::Applied {},
        Response::Error {
            code: ErrorCode::ConfirmationRequired,
            message: "use --force".into(),
        },
    ] {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &response).unwrap();
        let decoded: Response = read_frame(&mut bytes.as_slice()).unwrap();
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::to_value(decoded).unwrap()
        );
    }
}

#[test]
fn invalid_json_unknown_operations_and_unknown_fields_fail() {
    for bytes in [
        &b"not json"[..],
        br#"{"revision":1,"operation":{"op":"inject"}}"#,
        br#"{"revision":1,"operation":{"op":"list"},"extra":true}"#,
        br#"{"revision":1,"operation":{"op":"list","extra":true}}"#,
        br#"{"operation":{"op":"list"}}"#,
    ] {
        assert!(read_frame::<Request>(&mut framed(bytes).as_slice()).is_err());
    }
}

#[test]
fn unsupported_revision_and_invalid_ids_fail_validation() {
    assert!(
        Request {
            revision: 2,
            operation: Operation::List {}
        }
        .validate()
        .is_err()
    );
    for id in [
        "",
        "contains space",
        "../socket",
        "line\nbreak",
        &"x".repeat(129),
    ] {
        assert!(
            Request::new(Operation::Focus { id: id.into() })
                .validate()
                .is_err()
        );
    }
}

#[test]
fn malformed_frame_lengths_and_truncated_bodies_fail() {
    for bytes in [
        vec![],
        vec![0, 0],
        vec![0, 0, 0, 0],
        ((MAX_FRAME + 1) as u32).to_be_bytes().to_vec(),
        vec![0, 0, 0, 3, b'{'],
    ] {
        assert!(read_frame::<Request>(&mut bytes.as_slice()).is_err());
    }
    let mut writer = Vec::new();
    assert!(write_frame(&mut writer, &"x".repeat(MAX_FRAME)).is_err());
    assert!(writer.is_empty());
}

#[test]
fn directories_must_be_absolute_and_native() {
    let request = |cwd| {
        Request::new(Operation::Create {
            window: "i-w1".into(),
            name: None,
            cwd: Some(cwd),
        })
    };
    assert!(
        request(NativePath::from_path(std::path::Path::new("relative")))
            .validate()
            .is_err()
    );
    let absolute = std::env::current_dir().unwrap();
    request(NativePath::from_path(&absolute))
        .validate()
        .unwrap();
    assert_eq!(
        NativePath::from_path(&absolute).to_path().unwrap(),
        absolute
    );
    #[cfg(unix)]
    assert!(NativePath::Windows(vec![65]).to_path().is_err());
    #[cfg(windows)]
    assert!(NativePath::Unix(vec![65]).to_path().is_err());
}

#[test]
fn native_paths_reject_nul() {
    #[cfg(unix)]
    assert!(NativePath::Unix(vec![b'/', 0]).to_path().is_err());
    #[cfg(windows)]
    assert!(NativePath::Windows(vec![65, 0]).to_path().is_err());
}

#[test]
fn native_path_serialization_preserves_non_unicode_units() {
    #[cfg(unix)]
    let encoded = NativePath::Unix(vec![b'/', 255, b'a']);
    #[cfg(windows)]
    let encoded = NativePath::Windows(vec![67, 58, 92, 0xd800]);
    let bytes = serde_json::to_vec(&encoded).unwrap();
    let decoded: NativePath = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded, encoded);
    assert_eq!(NativePath::from_path(&decoded.to_path().unwrap()), encoded);
}
