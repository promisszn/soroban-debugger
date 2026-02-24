use soroban_env_host::Host;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    pub cpu_instructions: u64,
    pub memory_bytes: u64,
    pub wall_time: Duration,
}

pub struct ProfileSession {
    cpu_start: u64,
    mem_start: u64,
    start_time: Instant,
}

impl ProfileSession {
    pub fn start(host: &Host) -> Self {
        let budget = crate::inspector::budget::BudgetInspector::get_cpu_usage(host);

        Self {
            cpu_start: budget.cpu_instructions,
            mem_start: budget.memory_bytes,
            start_time: Instant::now(),
        }
    }

    pub fn finish(self, host: &Host) -> ExecutionMetrics {
        let budget_end = crate::inspector::budget::BudgetInspector::get_cpu_usage(host);

        ExecutionMetrics {
            cpu_instructions: budget_end.cpu_instructions.saturating_sub(self.cpu_start),
            memory_bytes: budget_end.memory_bytes.saturating_sub(self.mem_start),
            wall_time: self.start_time.elapsed(),
        }
    }
}
