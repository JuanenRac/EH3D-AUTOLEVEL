// EH3D-AUTOLEVEL - corrects the Z of a G-code program with the height map of the table.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! The height map itself (the grid, the probe readings and the bilinear surface) lives in `eh3d-levelling`, in EH3D-COMMON. This crate adds the bicubic surface and the
//! program that rewrites a G-code file so that the tool follows the table.

pub mod browse;
pub mod config;
pub mod gcode;
#[cfg(feature = "gui")]
pub mod gui;
pub mod lang;
pub mod surface;
pub mod viewport;
pub mod window;
