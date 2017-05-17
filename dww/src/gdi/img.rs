use user32;
use gdi32;
use winapi::*;

use ucs2::{WithString, UCS2_CONVERTER};
use dct::geometry::{Px, Rect, OriginRect};

use std::{ptr, mem, cmp, slice};
use std::path::Path;
use std::io::{Result, Error};

#[derive(Debug)]
pub struct DDBitmap( HBITMAP );
#[derive(Debug)]
pub struct DIBitmap( HBITMAP );
#[derive(Debug)]
pub struct DIBSection {
    handle: HBITMAP,
    bits: *mut [u8]
}
#[derive(Debug)]
pub struct Icon( HICON );

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitmapInfo {
    pub width: Px,
    pub height: Px,
    pub width_bytes: usize,
    pub bits_per_pixel: u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorFormat {
    BlackWhite,
    Paletted4,
    Paletted8,
    FullColor16,
    FullColor24,
    FullColor32
}

impl ColorFormat {
    pub fn bits_per_pixel(self) -> u8 {
        match self {
            ColorFormat::BlackWhite  => 1,
            ColorFormat::Paletted4   => 4,
            ColorFormat::Paletted8   => 8,
            ColorFormat::FullColor16 => 16,
            ColorFormat::FullColor24 => 24,
            ColorFormat::FullColor32 => 32
        }
    }
}


pub trait Bitmap {
    fn hbitmap(&self) -> HBITMAP;

    fn bitmap_info(&self) -> BitmapInfo {
        unsafe {
            let mut bitmap_struct: BITMAP = mem::zeroed();
            gdi32::GetObjectW(
                self.hbitmap() as HGDIOBJ,
                mem::size_of::<BITMAP>() as c_int,
                &mut bitmap_struct as *mut _ as *mut c_void
            );

            BitmapInfo {
                width: bitmap_struct.bmWidth,
                height: bitmap_struct.bmHeight,
                width_bytes: bitmap_struct.bmWidthBytes as usize,
                bits_per_pixel: bitmap_struct.bmBitsPixel as u8
            }
        }
    }

    fn bits(&self) -> &[u8] {
        unsafe {
            let mut bitmap_struct: BITMAP = mem::zeroed();
            gdi32::GetObjectW(
                self.hbitmap() as HGDIOBJ,
                mem::size_of::<BITMAP>() as c_int,
                &mut bitmap_struct as *mut _ as *mut c_void
            );

            slice::from_raw_parts(bitmap_struct.bmBits as *const u8, (bitmap_struct.bmHeight * bitmap_struct.bmWidthBytes) as usize)
        }
    }
}

impl DDBitmap {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<DDBitmap> {
        UCS2_CONVERTER.with_string(path.as_ref(), |path| {
            let bitmap = unsafe{ user32::LoadImageW(
                ptr::null_mut(), path.as_ptr(), IMAGE_BITMAP, 0, 0, LR_LOADFROMFILE
            )};

            if bitmap != ptr::null_mut() {
                Ok(DDBitmap(bitmap as HBITMAP))
            } else {
                Err(Error::last_os_error())
            }
        })
    }
}
impl Bitmap for DDBitmap {
    #[inline]
    fn hbitmap(&self) -> HBITMAP {
        self.0
    }
}

impl DIBitmap {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<DIBitmap> {
        UCS2_CONVERTER.with_string(path.as_ref(), |path| {
            let bitmap = unsafe{ user32::LoadImageW(
                ptr::null_mut(), path.as_ptr(), IMAGE_BITMAP, 0, 0,
                LR_LOADFROMFILE | LR_CREATEDIBSECTION
            )};

            if bitmap != ptr::null_mut() {
                Ok(DIBitmap(bitmap as HBITMAP))
            } else {
                Err(Error::last_os_error())
            }
        })
    }
}
impl Bitmap for DIBitmap {
    #[inline]
    fn hbitmap(&self) -> HBITMAP {
        self.0
    }
}

impl DIBSection {
    pub fn new(width: Px, height: Px, format: ColorFormat, x_ppm: Px, y_ppm: Px) -> DIBSection {
        unsafe {
            let (width, height, x_ppm, y_ppm) =
                (cmp::max(0, width), cmp::max(0, height), cmp::max(0, x_ppm), cmp::max(0, y_ppm));

            let bmp_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER  {
                    biSize: ::std::mem::size_of::<BITMAPINFO>() as u32,
                    biWidth: width,
                    biHeight: height,
                    biPlanes: 1,
                    biBitCount: format.bits_per_pixel() as u16,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: x_ppm,
                    biYPelsPerMeter: y_ppm,
                    biClrUsed: 0,
                    biClrImportant: 0
                },
                bmiColors: []
            };

            let mut pbits = ptr::null_mut();
            let hbitmap = gdi32::CreateDIBSection(
                ptr::null_mut(),
                &bmp_info,
                DIB_RGB_COLORS,
                &mut pbits,
                ptr::null_mut(),
                0
            );
            // This should only fail if there's an invalid parameter, which shouldn't happen if the
            // infrastructure code has been written properly.
            debug_assert_ne!(hbitmap, ptr::null_mut());

            let mut bitmap_struct: BITMAP = mem::zeroed();
            gdi32::GetObjectW(
                hbitmap as HGDIOBJ,
                mem::size_of::<BITMAP>() as c_int,
                &mut bitmap_struct as *mut _ as *mut c_void
            );

            let buffer_length = (bitmap_struct.bmWidthBytes * height) as usize;

            DIBSection {
                handle: hbitmap,
                bits: slice::from_raw_parts_mut(pbits as *mut u8, buffer_length)
            }
        }
    }

    pub fn bits_mut(&mut self) -> &mut [u8] {
        unsafe{ &mut *self.bits }
    }
}
impl Bitmap for DIBSection {
    #[inline]
    fn hbitmap(&self) -> HBITMAP {
        self.handle
    }
}

impl Icon {
    pub fn open<P: AsRef<Path>>(path: P, size: OriginRect) -> Result<Icon> {
        UCS2_CONVERTER.with_string(path.as_ref(), |path| {
            let icon = unsafe{ user32::LoadImageW(
                ptr::null_mut(), path.as_ptr(), IMAGE_ICON, size.width() as c_int,
                size.height() as c_int, LR_LOADFROMFILE
            )};

            if icon != ptr::null_mut() {
                Ok(Icon(icon as HICON))
            } else {
                Err(Error::last_os_error())
            }
        })
    }

    #[inline]
    pub fn hicon(&self) -> HICON {
        self.0
    }
}

impl Drop for DDBitmap {
    fn drop(&mut self) {
        unsafe{ gdi32::DeleteObject(self.0 as HGDIOBJ) };
    }
}
impl Drop for DIBitmap {
    fn drop(&mut self) {
        unsafe{ gdi32::DeleteObject(self.0 as HGDIOBJ) };
    }
}
impl Drop for DIBSection {
    fn drop(&mut self) {
        unsafe{ gdi32::DeleteObject(self.handle as HGDIOBJ) };
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        unsafe{ user32::DestroyIcon(self.0) };
    }
}
