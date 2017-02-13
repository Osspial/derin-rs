mod win32;

pub use self::win32::*;

use std::error::Error;
use std::fmt;

use std::default::Default;
use std::path::PathBuf;
use std::marker::{Send, Sync};

use dct::geometry::OriginRect;

#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// The window's name
    pub name: String,
    /// The window's dimensions
    pub size: Option<OriginRect>,

    /// Whether or not the window is a topmost window. If true, this window will
    /// always appear at the top of the Z order
    pub topmost: bool,

    /// Whether or not the window is a borderless window. Note that this will
    /// override any specified window decorations.
    pub borderless: bool,
    /// Whether or not the window can be resized by dragging on it's edge
    pub resizable: bool,
    /// Whether or not the window can be minimized
    pub maximizable: bool,
    /// Whether or not the window can be maximized
    pub minimizable: bool,
    /// Whether or not the window appears on the taskbar
    pub tool_window: bool,

    /// Whether or not the window can be transparent
    pub transparent: bool,

    /// The initial state of the window
    pub initial_state: InitialState,
    /// Whether or not to show the window upon creation
    pub show_window: bool,

    /// The path to the window's icon
    pub icon: Option<PathBuf>
}

unsafe impl Send for WindowConfig {}
unsafe impl Sync for WindowConfig {}

impl WindowConfig {
    /// Create a new window config. Identical to Default::default()
    #[inline]
    pub fn new() -> WindowConfig {
        Default::default()
    }

    #[inline]
    pub fn name(mut self, name: String) -> WindowConfig {
        self.name = name;
        self
    }

    #[inline]
    pub fn size(mut self, size: Option<OriginRect>) -> WindowConfig {
        self.size = size;
        self
    }


    #[inline]
    pub fn topmost(mut self, topmost: bool) -> WindowConfig {
        self.topmost = topmost;
        self
    }


    #[inline]
    pub fn borderless(mut self, borderless: bool) -> WindowConfig {
        self.borderless = borderless;
        self
    }

    #[inline]
    pub fn resizable(mut self, resizable: bool) -> WindowConfig {
        self.resizable = resizable;
        self
    }

    #[inline]
    pub fn maximizable(mut self, maximizable: bool) -> WindowConfig {
        self.maximizable = maximizable;
        self
    }

    #[inline]
    pub fn minimizable(mut self, minimizable: bool) -> WindowConfig {
        self.minimizable = minimizable;
        self
    }

    #[inline]
    pub fn tool_window(mut self, tool_window: bool) -> WindowConfig {
        self.tool_window = tool_window;
        self
    }


    #[inline]
    pub fn transparent(mut self, transparent: bool) -> WindowConfig {
        self.transparent = transparent;
        self
    }


    #[inline]
    pub fn initial_state(mut self, initial_state: InitialState) -> WindowConfig {
        self.initial_state = initial_state;
        self
    }

    #[inline]
    pub fn icon(mut self, icon: Option<PathBuf>) -> WindowConfig {
        self.icon = icon;
        self
    }
}


impl Default for WindowConfig {
    fn default() -> WindowConfig {
        WindowConfig {
            name: String::new(),
            size: None,

            topmost: false,

            borderless: false,
            resizable: true,
            maximizable: true,
            minimizable: true,
            tool_window: false,

            transparent: false,

            initial_state: InitialState::Windowed,
            show_window: true,

            icon: None
        }
    }
}

/// The initial state of the window
#[derive(Debug, Clone, Copy)]
pub enum InitialState {
    /// The window starts as a floating window
    Windowed,
    /// The window starts minimized
    Minimized,
    /// The window starts maximized
    Maximized
}

pub type NativeResult<T> = Result<T, NativeError>;

#[derive(Debug, Clone)]
pub enum NativeError {
    OsError(String),
    IconLoadError(u16)
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::NativeError::*;

        match *self {
            OsError(ref s) => write!(f, "{}", s),
            IconLoadError(size) => write!(f, "Could not load {0}x{0} icon", size)
        }
    }
}

impl Error for NativeError {
    fn description<'a>(&'a self) -> &'a str {
        use self::NativeError::*;

        match *self {
            OsError(ref s) => s,
            IconLoadError(_) => "Icon load error"
        }
    }
}
