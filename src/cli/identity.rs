use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};

pub fn instance() -> &'static str {
    static INSTANCE: OnceLock<String> = OnceLock::new();
    INSTANCE.get_or_init(|| uuid::Uuid::new_v4().simple().to_string())
}

pub fn next(kind: char) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let number = NEXT.fetch_add(1, Ordering::Relaxed);
    assert_ne!(number, u64::MAX, "cli: runtime ID counter exhausted");
    format!("{}-{kind}{number}", instance())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_process_scoped_unique_and_do_not_depend_on_order() {
        let first = next('s');
        let second = next('s');
        let window = next('w');
        let mut sessions = [first.clone(), second.clone()];
        sessions.swap(0, 1);
        assert_eq!(sessions[1], first);
        assert_eq!(sessions[0], second);
        assert_ne!(first, second);
        assert!(first.starts_with(instance()));
        assert!(window.starts_with(instance()));
        let generated: HashSet<_> = (0..1000).map(|_| next('s')).collect();
        assert_eq!(generated.len(), 1000);
    }
}
