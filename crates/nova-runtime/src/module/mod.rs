// Nova Module System — Transparent Hot-Reloading
//
// Architecture (Blue-Green Module Swap):
//
//   ┌──────────────┐    ┌──────────────┐
//   │  Module v1   │    │  Module v2   │
//   │  (active)    │    │  (staged)    │
//   │              │    │              │
//   │  state: Run  │    │  state: Ready│
//   └──────┬───────┘    └──────┬───────┘
//          │                   │
//          ▼                   ▼
//   ┌─────────────────────────────────┐
//   │        Message Router           │
//   │  • New calls → v2 (staged)     │
//   │  • Active calls → v1 (drain)   │
//   │  • v1 drained → swap complete  │
//   └─────────────────────────────────┘
//
// The developer writes normal code. The compiler splits it
// into modules based on dependency analysis. At runtime,
// the module manager handles versioning transparently.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// The lifecycle state of a module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Module is loading/compiling
    Loading,
    /// Module is compiled and ready to accept calls
    Ready,
    /// Module is actively handling calls
    Running,
    /// Module is idle (no active calls)
    Idle,
    /// Module is being replaced — draining active calls
    Draining,
    /// Module has been replaced and is waiting for cleanup
    Retired,
    /// Module encountered an error
    Error,
}

/// A single module in the Nova runtime
#[derive(Debug)]
pub struct Module {
    /// Unique module identifier (derived from source location)
    pub id: String,
    /// Current lifecycle state
    pub state: ModuleState,
    /// Number of currently active calls into this module
    pub active_calls: usize,
    /// Module version (incremented on each reload)
    pub version: u64,
    /// Timestamp of last state change
    pub last_changed: Instant,
    /// Dependencies — other module IDs this module calls
    pub dependencies: Vec<String>,
}

/// The module manager handles all loaded modules and
/// orchestrates hot-reloading via blue-green swaps.
pub struct ModuleManager {
    modules: Arc<RwLock<HashMap<String, Module>>>,
    /// Staged modules waiting to replace active ones
    staged: Arc<RwLock<HashMap<String, Module>>>,
}

impl Module {
    pub fn new(id: String) -> Self {
        Self {
            id,
            state: ModuleState::Loading,
            active_calls: 0,
            version: 1,
            last_changed: Instant::now(),
            dependencies: Vec::new(),
        }
    }

    /// Record that a call has entered this module
    pub fn enter_call(&mut self) {
        self.active_calls += 1;
        if self.state == ModuleState::Idle {
            self.transition(ModuleState::Running);
        }
    }

    /// Record that a call has exited this module
    pub fn exit_call(&mut self) {
        self.active_calls = self.active_calls.saturating_sub(1);
        if self.active_calls == 0 {
            match self.state {
                ModuleState::Running => self.transition(ModuleState::Idle),
                ModuleState::Draining => self.transition(ModuleState::Retired),
                _ => {}
            }
        }
    }

    /// Transition to a new state
    fn transition(&mut self, new_state: ModuleState) {
        self.state = new_state;
        self.last_changed = Instant::now();
    }

    /// Begin draining — no new calls, wait for existing to finish
    pub fn begin_drain(&mut self) {
        if self.active_calls == 0 {
            self.transition(ModuleState::Retired);
        } else {
            self.transition(ModuleState::Draining);
        }
    }

    /// Check if this module can be safely removed
    pub fn is_drained(&self) -> bool {
        self.state == ModuleState::Retired && self.active_calls == 0
    }
}

impl ModuleManager {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            staged: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new module
    pub fn register(&self, mut module: Module) {
        module.state = ModuleState::Idle;
        let id = module.id.clone();
        self.modules.write().unwrap().insert(id, module);
    }

    /// Stage a new version of a module for hot-swap
    pub fn stage_update(&self, mut new_module: Module) {
        new_module.state = ModuleState::Ready;
        let id = new_module.id.clone();
        self.staged.write().unwrap().insert(id, new_module);
    }

    /// Execute the hot-swap for a staged module.
    ///
    /// 1. The old module begins draining
    /// 2. New calls are routed to the staged version
    /// 3. Once the old module is fully drained, it is retired
    pub fn swap(&self, module_id: &str) -> Result<(), ModuleSwapError> {
        let mut staged = self.staged.write().unwrap();
        let new_module = staged
            .remove(module_id)
            .ok_or(ModuleSwapError::NoStagedVersion)?;

        let mut modules = self.modules.write().unwrap();

        // Begin draining the old module
        if let Some(old) = modules.get_mut(module_id) {
            old.begin_drain();
            // In a real implementation, we'd keep the old module
            // accessible until fully drained. For now, we replace.
        }

        modules.insert(module_id.to_string(), new_module);
        Ok(())
    }

    /// Get current state of all modules
    pub fn status(&self) -> Vec<(String, ModuleState, usize)> {
        self.modules
            .read()
            .unwrap()
            .iter()
            .map(|(id, m)| (id.clone(), m.state, m.active_calls))
            .collect()
    }
}

impl Default for ModuleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleSwapError {
    #[error("no staged version available for swap")]
    NoStagedVersion,
    #[error("module is in an invalid state for swapping: {0:?}")]
    InvalidState(ModuleState),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_lifecycle() {
        let mut module = Module::new("test_mod".into());
        assert_eq!(module.state, ModuleState::Loading);

        module.state = ModuleState::Idle;
        module.enter_call();
        assert_eq!(module.state, ModuleState::Running);
        assert_eq!(module.active_calls, 1);

        module.exit_call();
        assert_eq!(module.state, ModuleState::Idle);
        assert_eq!(module.active_calls, 0);
    }

    #[test]
    fn test_module_drain() {
        let mut module = Module::new("test_mod".into());
        module.state = ModuleState::Running;
        module.active_calls = 2;

        module.begin_drain();
        assert_eq!(module.state, ModuleState::Draining);

        module.exit_call();
        assert_eq!(module.state, ModuleState::Draining);

        module.exit_call();
        assert_eq!(module.state, ModuleState::Retired);
        assert!(module.is_drained());
    }

    #[test]
    fn test_hot_swap() {
        let manager = ModuleManager::new();

        let v1 = Module::new("math".into());
        manager.register(v1);

        let mut v2 = Module::new("math".into());
        v2.version = 2;
        manager.stage_update(v2);

        manager.swap("math").unwrap();

        let status = manager.status();
        let math = status.iter().find(|(id, _, _)| id == "math").unwrap();
        assert_eq!(math.1, ModuleState::Ready);
    }
}
