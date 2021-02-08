//! UI system.
//!
//! # Design Rationale
//!
//! Crawl's UI is optimized for three situations:
//! - An 80 x 24 terminal because it would be silly not to support something so
//!   ancient.
//! - A full-screen terminal on a laptop. Let's just say this has 2.5 the
//!   dimensions of a 80 x 24 (i.e., 200 x 56).
//! - A terminal the size of my ultra-ultra wide monitor, which is apparently
//!   600 x 80.
//!
//! In the former, screen real estate is minimal and maximizing the amount of
//! "game" visible is ideal. In the latter, it would be good to keep UI elements
//! close to the center of the terminal (the "camera" is always locked to this
//! position), but cramming things into a 80 x 24 is really silly.
//!
//! Fundamentally the Crawl UI is composed of five kinds of UI components:
//! - The game itself, at the lowest layer.
//! - Left and right "curtain" windows (analogous to the curtains on a stage).
//!   These appear on the outer portions of the game window.
//! - HUD "widgets". These are up to two rows of widgets displayed at the bottom
//!   (or top, as a group) such as HP and money. These take up the entire bottom
//!   of the screen.
//! - Banners, which fit between the curtains at the opposite side from the HUD.
//! - Infoboxes, which can be placed anywhere and which are above all other
//!   UI components.
//!
//! The UI defines two "reference" terminal sizes: SD (80 x 24) and HD
//! (200 x 56). Smaller than SD is not supported; larger than HD looks like HD
//! centered on the center of the terminal but with extra map visible beyond the
//! normal terminal boundaries. Between SD and HD we attempt to smoothly
//! interpolate. At no point should the curtains, when unfocused, take up more
//! than 30% of the screen cumulatively.
//!
//! Throughout the discussion below, the terminal is always assumed to be
//! between SD and HD in size.
//!
//! # Widgets
//!
//! The HUD widgets' boundaries are not "defined" by anything else, so we define
//! them first. The HUD consists of two rows at the bottom of the screen made up
//! of "widgets". A widget is simply a one-character-high rectangle with a
//! minimum and maximum length, which are laid out in such a way as to fairly
//! maximize the length of each.
//!
//! The HUD can be hidden, the rows can be swapped for each other, and can be
//! placed at the top if desired (this moves splashes to the bottom of the
//! screen).
//!
//! # Curtains
//!
//! A "curtain" is a window on the left or right border of the screen. A
//! curtain's height is always the terminal height minus 2 for the HUD.
//! A curtain's width can range between 20 and 40 characters, including the
//! border.
//!
//! Curtains can be hidden, on standby, focused, or maximized, which have
//! different effects on their width:
//! - Minimized curtains are effectively hidden, drawn minimally to show
//!   whatever key can open them.
//! - Standby curtains are 15% of the terminal's current width. If that's less
//!   than 20 characters, the curtain is minimized; if it's more than 40, it's
//!   clamped at 40.
//! - Focused curtains are 15% of the terminal's current width, clamped between
//!   20 and 40 characters.
//! - Maximized curtains are always 40 characters.
//!
//! Beyond HD, standby, focused, and maximized are all the same.
//!
//! Curtains are effectively a pair of browser windows for Crawl's menus, which
//! display scrollable hypertext. Each curtain may be split in half to display
//! two menus, though they maximize and minimize together.
//!
//! # Banners and Infoboxes
//!
//! Banners occupy the top five lines of the screen (or bottom, if the HUD is
//! inverted). Infoboxes can be anywhere.
//!

pub mod widget;
