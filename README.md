<img width="1120" height="480" alt="rat-bar-thumbnail" src="https://github.com/user-attachments/assets/1fcfeba8-3630-4ecb-bd06-f6ee7ecefd84" />

# RAT-BAR

A terminal based status bar built in rust with [ratatui](https://ratatui.rs/). It provides various built in widgets such as music player status, an audio visualizer and much more. Custom widgets can also display info provided by any program which can output json formatted lines.

# Quickstart

Currently there are 2 files (`providers.yaml` and `layout.yaml`) that need to be present in `~/.config/rat-bar` for rat-bar to start. 
If you are using the example config you also need `providers.nu` in the same folder.
The tested way to use rat-bar is with Kitty's [`kitten panel`](https://sw.kovidgoyal.net/kitty/kittens/panel/) and the convenience script `scripts.nu spawn all`.

Example config files can be found in the repository, but they **REQUIRE** nushell to be installed and hardware values like net adapter in `providers.yaml` need to match your machine.
The example nushell scripts can be replaced by anything that periodically outputs json delimited by \n:

```json
{"foo": 1, "bar": "my"}\n
(sleep 1s)
{"foo": 3, "bar": "custom"}\n
(sleep 1s)
{"foo": 5, "bar": "provider"}\n
```

Additional dependencies are `nvidia-smi` for `nvidia` functionality and `playerctl` for `now-playing`, however all other providers should just work.

<details>

<summary>

## Nix

</summary>

This repo includes a flake, if you want to try it out simply copy `providers.yaml` `layout.yaml` and `providers.nu` into `~/.config/rat-bar/` and make sure you have nu installed!

`nix shell 'nixpkgs#nushell' 'github:van-nessing/rat-bar#rat-bar-scripts' -c rat-bar-scripts spawn all`

### Using with Home Manager

The repo also includes a home module which allows you to configure your bar declaratively and start it automatically.

#### Default setup

```nix
# flake.nix
{
  inputs = {
    rat-bar = {
      url = "github:van-nessing/rat-bar";
      inputs.nixpkgs.follows = "nixpkgs";
    }
  }
}
```

```nix
# home.nix
{
  imports = [ inputs.rat-bar.homeModules.default ];
  # Include to resize with `rat-bar-scripts resize`
  home.packages = with pkgs; [ rat-bar-scripts ];

  # Enables auto start service with default config
  programs.rat-bar = {
    enable = true;

    # Disables service
    # service.enable = false

    # Changes default height
    # service.height = 3
  }
}
```

#### Customization

You can replace the custom providers like this (just make sure to enable all the ones used in your layout):

```nix
# home.nix
# Custom providers
{
  programs.rat-bar = {
     providers =
     let
       providers = lib.getExe pkgs.rat-bar-providers;
     in
     {      
       cpu.command = [
         providers
         "cpu"
         "1sec"
         "" # Insert temperature sensor name (nu -c 'sys temp')
         "12"
       ];
       nvidia.command = [
         providers
         "nvidia"
         "1sec"
         "12"
       ];
       my-provider.command = [
         /path/to/binary
         "my"
         "args"
       ];
       # ... And all the other providers used by your layout
    };
  }
}

```nix
# home.nix
# Custom layout
{
  programs.rat-bar = {
    layout =
    # Helpful functions for doing layout:
    let
      type = type: attrs: { ${type} = attrs; };
      text = t: type "Text" t;
      mod = prev: mod: builtins.mapAttrs (key: val: val // mod) prev;
      width = width: prev: mod prev { inherit width; };
      no-center = prev: mod prev { center = false; };
      vgroup =
        elements:
        type "VGroup" {
          inherit elements;
        };
      hgroup =
        elements:
        type "HGroup" {
          inherit elements;
        };
      bar =
        var: direction:
        type "Bar" {
          inherit var direction;
        };
      graph =
        var:
        type "Graph" {
          inherit var;
        };
      image =
        var: width:
        type "Image" {
          inherit var width;
        };
    in
    [
      {
        block.title = "GPU";
        constraint = { Length = 35 };
        component_type = provider {
          provider = "nvidia";
          layout = [
            (hgroup [
              (vgroup [
                (text "LOAD")
                (text "\${utilization.gpu}%")
              ])
              (vgroup [
                (text "USED")
                (text "\${memory.used}GB")
              ])
              (vgroup [
                (text "USED")
                (text "\${memory.free}GB")
              ])
              # Force width of 2 characters
              (width { Lenght = 2; } (bar "percent" "Vertical"))
              # If you have the pipe-operators feature enabled:
              # (bar "percent" "Vertical" |> width { Length = 2; })
              (graph "acc")
            ])
          ];
        };
      }
      # ... rest of your layout
    ];
  }
}
```

</details>

# Layout

```yaml
- block:
    title: "my title"
  constraint:
    # Use Length Percentage Fill etc...
    Length: 35
  component_type:
    Provider:
      # Name of provider in providers.yaml
      provider: "my-provider"
      # Depending on the height of space available the matching element will get selected
      # Useful if you want a taller bar that shows more info on your main screen
      # Resize using `rat-bar-scripts resize`
      layout:
      # Height 1
      - Text: "${my_var} and ${other_var}"
      # Height 2
      - VGroup:
          elements:
          - Text: "${my_var}"
          - Text: "${other_var}"
    # etc...
# blocks are optional but usually you want them
- component_type: !Provider
  constraint:
    Percentage: 30
```

`layout.yaml` defines the layout of the bar.
Components have a `block` option which wraps them in a block with `title` and a `constraint` option which controls how wide the component will be. Valid options for `constraint` are `Length`, `Percentage`, `Fill`, `Min`, `Max` but `Length` and `Percentage` are by far the most useful ones.
Read the example configuration in the repo to see what's possible! I left some comments to document what's happening

| Component | Description |
| --------- | ----------- |
| `Group`   | Groups its elements together, can be used to make nested blocks |
| `Provider`| The most powerful component. A provider is a program that gets invoked by rat-bar and sends data to rat-bar |
| `Visualizer` | Displays an spectrum audio visualizer using pipewire |

## Components

### `Provider`

The `Provider` component uses variables supplied by the specified `provider` to display text, graphs and bars. The `provider` field decides which provider in `providers.yaml` to get its variables from.

#### Config

`providers.yaml` maps the provider name used in `layout.yaml` to a command that will get executed when the bar starts up

```yaml
clock:
  command:
    - nu
    - ~/.config/rat-bar/provider.nu
    - clock
    - --interval 1sec
cpu:
  command:
    - nu
    - ~/.config/rat-bar/provider.nu
    - cpu
    - --interval 1sec
    - --temp_sensor 'k10temp Tccd1'
    - --acc_count 12
```

#### Provider Layout

| Element | Description |
| ------- | ----------- |
| `HGroup`| Displays `elements` in a row |
| `VGroup`| Displays `elements` in a row, elements can be centered with `center: true` |
| `Text`  | Displays text, can contain provider variables using `${var_name}` syntax and can get styled using `$style_name(foo: ${some_var} bar)` (style options are `ul` for underlining) |
| `Bar`   | Displays a bar in `direction` (either `Horizontal` or `Vertical`) using `var` ranging from 0-100 |
| `Graph` | Displays a graph using `var` which contains a list of values ranging from 0-100 | 
| `Image` | Displays an image using `var` which contains the path to an image |

Additionally each element type except `Text` has an optional `width` field

### `Visualizer`

`Visualizer` uses pipewire to get the audio output and construct an averaged spectrum view. The channels and format are hardcoded and I have no clue if it works on machines other than mine.

### `Group`

`Group` is a leftover from when I started development and will probably get removed eventually.
