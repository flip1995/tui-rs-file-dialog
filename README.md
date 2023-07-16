# tui-rs File Dialog

This is a tui-rs extension for a file dialog popup.

## Usage

See the `examples` directory on how to use this extension. Run

```
cargo run --example demo
```

to see it in action.

First, add a file dialog to the TUI app:

```rust
use tui_rs_file_dialog::FileDialog;

struct App {
    // Other fields of the App...

    file_dialog: FileDialog
}
```

If you want to use the default key bindings provided by this crate, just wrap
the event handler of your app in the `bind_keys!` macro:

```rust
use tui_rs_file_dialog::bind_keys;

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        bind_keys!(
            // Expression to use to access the file dialog.
            app.file_dialog,
            // Event handler of the app, when the file dialog is closed.
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        )
    }
}
```

Finally, draw the file dialog:

```rust
fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Other UI drawing code...

    app.file_dialog.draw(f);
}
```

## Limitations

I've started this crate with a minimalistic approach and new functionality will
be added on a use-case basis. For example, it is currently not possible to add
styling to the file dialog and just a boring, minimalist block with a list is
used to render it.

## Contribution

This crate is developed on a use-case basis. If you want to also use this crate,
but it currently doesn't cover your use case, feel free to open an issue or even
a PR for it. I'm open to ideas and improvements.

## License

Copyright 2023 Philipp Krones

Licensed under the MIT license. Files in the project may not be copied,
modified, or distributed except according to those terms.
