//! Both sides of the small-batch crossover preserve ordering and deduplication.

use super::*;

#[test]
fn twelve_and_thirteen_incoming_names_keep_existing_work_first() {
    for size in [12, 13] {
        let pool = Pool::new();
        let active = [Path::new("/work"), Path::new("/work")];
        pool.update_queue(active, [Some(batch("/work", &["existing"])), None]);
        let mut names = vec![FileName::from("existing")];
        names.extend((0..size - 1).map(|i| FileName::new(format!("repo-{i}"))));
        let incoming = Some((PathBuf::from("/work"), names.clone()));
        assert_eq!(pool.update_queue(active, [incoming, None]), size);
        let mut expected_queue = vec![expected("/work", "existing")];
        expected_queue.extend(
            names
                .into_iter()
                .skip(1)
                .map(|name| (PathBuf::from("/work"), name)),
        );
        assert_eq!(queued(&pool), expected_queue);
    }
}
