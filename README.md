# mutt-ics-reader

Read iCalendar (`.ics`) invitations in the terminal. It shows the events of a
file in a small TUI, or prints them as plain text so mutt and NeoMutt can
display an invitation inline.

Dates are converted to your local time zone, including the Windows time zone
names that Outlook and Exchange put in their invitations, both as keys
(`Romance Standard Time`, `Pacific Standard Time`, ...) and as display names
(`(UTC+01:00) Brussels, Copenhagen, Madrid, Paris`, ...).

## Install

You need a Rust toolchain. Get one from <https://rustup.rs> if you don't have it.

```sh
cargo install --path .
```

This builds the release binary and puts `mutt-ics-reader` in `~/.cargo/bin`,
which `rustup` adds to your `PATH`. To build without installing:

```sh
cargo build --release
./target/release/mutt-ics-reader invitation.ics
```

## Usage

```
mutt-ics-reader [--plain] <file.ics>
```

Without `--plain`, a two-pane TUI opens: the event list on the left, the
details of the selected event on the right.

| Key | Action |
|---|---|
| `j` / `↓` | next event |
| `k` / `↑` | previous event |
| `q` / `Esc` | quit |

With `--plain`, the events are printed to stdout with ANSI colors and the
program exits. Set the `NO_COLOR` environment variable to disable the colors.

## NeoMutt / mutt setup

Two mailcap entries per MIME type, in `~/.mailcap`: one interactive, one for
inline display. NeoMutt picks the `copiousoutput` entry when it renders the
message body and the `needsterminal` entry when you open the attachment
explicitly.

```
text/calendar;    mutt-ics-reader --plain %s; copiousoutput
text/calendar;    mutt-ics-reader %s; needsterminal
application/ics;  mutt-ics-reader --plain %s; copiousoutput
application/ics;  mutt-ics-reader %s; needsterminal
```

Then in `~/.neomuttrc` (or `~/.muttrc`):

```
# Render invitations inline in the pager
auto_view text/calendar application/ics

# Outlook sends the invitation as one branch of a multipart/alternative.
# Prefer it over the HTML and text bodies.
alternative_order text/calendar text/html text/plain

# Keep the colors of the plain output
set allow_ansi = yes
```

To open the TUI from a message, press `v` to list the attachments, select the
`.ics` part and press Enter.

Optionally, highlight invitations in the index:

```
color index color171 default "~M text/calendar"
```

`~M` reads each message to inspect its MIME parts, which NeoMutt documents as
a slow pattern. On a local Maildir the cost is barely noticeable.

## License

Apache License 2.0, see [LICENSE](LICENSE).
