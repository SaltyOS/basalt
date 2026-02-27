//! Compatibility re-export of Besalt UAPI constants.
//! SPDX-License-Identifier: GPL-2.0-only
//!
//! The canonical source of truth now lives in `besalt/uapi/rust/consts.rs`.
//! Keep this module as a thin forwarding layer while userland is migrated.

include!("../../uapi/rust/consts.rs");
