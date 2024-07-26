use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use super::Task;
use alloc::collections::VecDeque;

pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    #[must_use]
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task);
    }

    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {}                             // task is finished
                Poll::Pending => self.task_queue.push_back(task), // push back and check later
            }
        }
    }
}

impl Default for SimpleExecutor {
    fn default() -> Self {
        SimpleExecutor::new()
    }
}

/// Creates a dummy [`RawWaker`] that is effectively doing nothing.
fn raw_dummy_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        raw_dummy_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    #[allow(clippy::zero_ptr)] // passed data does not matter because everything is no_op
    RawWaker::new(0 as *const (), vtable)
}

fn dummy_waker() -> Waker {
    // Safety: contract is upheld.
    unsafe { Waker::from_raw(raw_dummy_waker()) }
}
