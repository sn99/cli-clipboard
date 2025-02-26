/*
Copyright 2019 Gregory Meyer

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use crate::common::*;
use crate::Result;
use std::io::{self, Read};
use wl_clipboard_rs::{
    copy::{self, clear, Options, ServeRequests},
    paste, utils,
};

/// Interface to the clipboard for Wayland windowing systems.
///
/// Other users of the Wayland clipboard will only see the contents
/// copied to the clipboard so long as the process copying to the
/// clipboard exists. If you need the contents of the clipboard to
/// remain after your application shuts down, consider daemonizing the
/// clipboard components of your application.
///
/// `WaylandClipboardContext` automatically detects support for and
/// uses the primary selection protocol.
///
/// # Example
///
/// ```noop
/// use cli_clipboard::ClipboardProvider;
/// let mut clipboard = cli_clipboard::wayland_clipboard::WaylandClipboardContext::new().unwrap();
/// clipboard.set_contents("foo bar baz".to_string()).unwrap();
/// let contents = clipboard.get_contents().unwrap();
///
/// assert_eq!(contents, "foo bar baz");
/// ```
pub struct WaylandClipboardContext {
    supports_primary_selection: bool,
}

impl ClipboardProvider for WaylandClipboardContext {
    /// Constructs a new `WaylandClipboardContext` that operates on all
    /// seats using the data-control clipboard protocol.  This is
    /// intended for CLI applications that do not create Wayland
    /// windows.
    ///
    /// Attempts to detect whether the primary selection is supported.
    /// Assumes no primary selection support if no seats are available.
    /// In addition to returning Err on communication errors (such as
    /// when operating in an X11 environment), will also return Err if
    /// the compositor does not support the data-control protocol.
    fn new() -> Result<WaylandClipboardContext> {
        let supports_primary_selection = match utils::is_primary_selection_supported() {
            Ok(v) => v,
            Err(utils::PrimarySelectionCheckError::NoSeats) => false,
            Err(e) => return Err(e.into()),
        };

        Ok(WaylandClipboardContext {
            supports_primary_selection,
        })
    }

    /// Pastes from the Wayland clipboard.
    ///
    /// If the Wayland environment supported the primary selection when
    /// this context was constructed, first checks the primary
    /// selection. If pasting from the primary selection raises an
    /// error or the primary selection is unsupported, falls back to
    /// the regular clipboard.
    ///
    /// An empty clipboard is not considered an error, but the
    /// clipboard must indicate a text MIME type and the contained text
    /// must be valid UTF-8.
    fn get_contents(&mut self) -> Result<String> {
        if self.supports_primary_selection {
            match paste::get_contents(
                paste::ClipboardType::Primary,
                paste::Seat::Unspecified,
                paste::MimeType::Text,
            ) {
                Ok((mut reader, _)) => {
                    // this looks weird, but rustc won't let me do it
                    // the natural way
                    return Ok(read_into_string(&mut reader).map_err(Box::new)?);
                }
                Err(e) => match e {
                    paste::Error::NoSeats
                    | paste::Error::ClipboardEmpty
                    | paste::Error::NoMimeType => return Ok("".to_string()),
                    _ => (),
                },
            }
        }

        let mut reader = match paste::get_contents(
            paste::ClipboardType::Regular,
            paste::Seat::Unspecified,
            paste::MimeType::Text,
        ) {
            Ok((reader, _)) => reader,
            Err(
                paste::Error::NoSeats | paste::Error::ClipboardEmpty | paste::Error::NoMimeType,
            ) => return Ok("".to_string()),
            Err(e) => return Err(e.into()),
        };

        Ok(read_into_string(&mut reader).map_err(Box::new)?)
    }

    /// Copies to the Wayland clipboard.
    ///
    /// If the Wayland environment supported the primary selection when
    /// this context was constructed, this will copy to both the
    /// primary selection and the regular clipboard. Otherwise, only
    /// the regular clipboard will be pasted to.
    fn set_contents(&mut self, data: String) -> Result<()> {
        let mut options = Options::new();

        options
            .seat(copy::Seat::All)
            .trim_newline(false)
            .foreground(false)
            .serve_requests(ServeRequests::Unlimited);

        if self.supports_primary_selection {
            options.clipboard(copy::ClipboardType::Both);
        } else {
            options.clipboard(copy::ClipboardType::Regular);
        }

        options
            .copy(
                copy::Source::Bytes(data.into_bytes().into()),
                copy::MimeType::Text,
            )
            .map_err(Into::into)
    }

    fn clear(&mut self) -> Result<()> {
        if self.supports_primary_selection {
            clear(copy::ClipboardType::Both, copy::Seat::All).map_err(Into::into)
        } else {
            clear(copy::ClipboardType::Regular, copy::Seat::All).map_err(Into::into)
        }
    }
}

fn read_into_string<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn wayland_test() {
        let mut clipboard =
            WaylandClipboardContext::new().expect("couldn't create a Wayland clipboard");

        clipboard
            .set_contents("foo bar baz".to_string())
            .expect("couldn't set contents of Wayland clipboard");

        assert_eq!(
            clipboard
                .get_contents()
                .expect("couldn't get contents of Wayland clipboard"),
            "foo bar baz"
        );
    }
}
