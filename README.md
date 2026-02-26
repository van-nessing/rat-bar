<img width="1120" height="480" alt="rat-bar-thumbnail" src="https://github.com/user-attachments/assets/1fcfeba8-3630-4ecb-bd06-f6ee7ecefd84" />

# RAT-BAR

A terminal based status bar built in rust with [ratatui](https://ratatui.rs/). It provides various built in widgets such as music player status, an audio visualizer and much more. Custom widgets can also display info provided by any program which can output json formatted lines.

# Quickstart

Currently there are 2 files (`providers.yaml` and `layout.yaml`) that need to be present in `~/.config/rat-bar` for rat-bar to start. 

The tested way to use rat-bar is with Kitty's [`kitten panel`](https://sw.kovidgoyal.net/kitty/kittens/panel/) and the convenience script `scripts.nu`. Because of nushell quirks or my own inability I could not get all bars to close when spawning them via jobs, so for the time being use `scripts.nu spawn-sh` which constructs a bash command that launches the bar on all monitors and kills them when the parent process exits.

Example config files can be found in the repository, but they **REQUIRE** nushell to be installed and hardware values like net adapter in `providers.yaml` need to match your machine.
The nushell scripts can be replaced by anything that periodically outputs json delimited by \n:

```json
{"foo": 1, "bar": "my"}\n
(sleep 1s)
{"foo": 3, "bar": "custom"}\n
(sleep 1s)
{"foo": 5, "bar": "provider"}\n
```

# Layout

```yaml
component_type: !Group
  components:
    - block:
        title: "title"
      constraint: !Length 20
      component_type: !Provider
        etc...
    - block:
        title: "other title"
      constraint: !Percentage 50
      component_type: !NowPlaying
        etc...
```

`layout.yaml` defines the layout of the bar. Usually you have `Group` as a top level element, which contains all your other bar components such as `Visualizer` and `NowPlaying`.
Components have a `block` option which wraps them in a block with `title` and a `constraint` option which controls how wide the component will be. Valid options for `constraint` are `Length`, `Percentage`, `Fill`, `Min`, `Max` but `Length` and `Percentage` are by far the most useful ones.  

| Component | Description |
| --------- | ----------- |
| `Group`   | Groups its elements together, can be used to make nested blocks |
| `Provider`| The most powerful component. A provider is a program that gets invoked by rat-bar and sends data to rat-bar |
| `NowPlaying` | Uses the mpris2 d-bus interface to display info about currently playing media with scrolling text |
| `Visualizer` | Displays an spectrum audio visualizer using pipewire |

## Components

### `Provider`

The `Provider` component uses variables supplied by the specified `provider` to display text, graphs and bars. The `provider` field decides which provider in `providers.yaml` to get its variables from.

#### Provider Layout

| Element | Description |
| ------- | ----------- |
| `HGroup`| Displays `elements` in a row |
| `VGroup`| Displays `elements` in a row, elements can be centered with `center: true` |
| `Text`  | Displays text, can contain provider variables using `${var_name}` syntax
| `Bar`   | Displays a bar in `direction` (either `Horizontal` or `Vertical`) using `var` ranging from 0-100
| `Graph` | Displays a graph using `var` which contains a list of values ranging from 0-100

Additionally each element type except `Text` has an optional `width` field

### `NowPlaying`

The `NowPlaying` component displays title, artist and album and the album cover in a music player style. Text will scroll when it does not fit in the available space.

### `Visualizer`

`Visualizer` uses pipewire to get the audio output and construct an averaged spectrum view. The channels and format are hardcoded and I have no clue if it works on machines other than mine.

### `Group`

`Group` is a leftover from when I started development and will probably get removed eventually.
