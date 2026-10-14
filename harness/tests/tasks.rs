//! Every experiment task folder loads, with 2 visible and 8 hidden cases and at least one error case.

use harness::task::Task;
use std::fs;
use std::path::Path;

#[test]
fn every_experiment_task_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../experiment/tasks");
    let mut count = 0;
    for entry in fs::read_dir(&root).unwrap() {
        let dir = entry.unwrap().path();
        let task = Task::load(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        assert_eq!(task.visible.len(), 2, "{}", dir.display());
        assert_eq!(task.hidden.len(), 8, "{}", dir.display());
        let errors = task.hidden.iter().filter(|c| c.status == 20).count();
        assert!(errors > 0, "{}: no error case", dir.display());
        count += 1;
    }
    assert!(count > 0, "no task folder");
}
