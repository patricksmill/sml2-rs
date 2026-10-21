//! Wario boss forms, children, and phase transitions.

mod child22;
mod child25;
mod child_fall;
mod drop_child;
mod drop_child_adapter;
#[cfg(test)]
#[path = "tests/drop_child.rs"]
mod drop_child_tests;
mod form1;
mod form1_adapter;
#[cfg(test)]
#[path = "tests/form1_integration.rs"]
mod form1_integration;
mod form2;
mod form2_adapter;
#[cfg(test)]
#[path = "tests/form2_integration.rs"]
mod form2_integration;
#[cfg(test)]
#[path = "tests/form2.rs"]
mod form2_tests;
mod form3;
mod form3_adapter;
#[cfg(test)]
#[path = "tests/form3_integration.rs"]
mod form3_integration;
#[cfg(test)]
#[path = "tests/form3.rs"]
mod form3_tests;
mod moving_child;
mod moving_child_adapter;
mod phase1_exit;
mod phase2_exit;
mod trajectory_child;
mod trajectory_child_adapter;
#[cfg(test)]
#[path = "tests/trajectory_child.rs"]
mod trajectory_child_tests;
