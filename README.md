<img width="1120" height="480" alt="rat-bar-thumbnail" src="https://github.com/user-attachments/assets/1fcfeba8-3630-4ecb-bd06-f6ee7ecefd84" />

# RAT-BAR

A terminal based status bar built in rust with [ratatui](https://ratatui.rs/). It provides various built in widgets such as music player status, an audio visualizer and much more. Custom widgets can also display info provided by any program which can output json formatted lines.

# Quickstart

```sh
mkdir -f ~/.config/rat-bar
git clone https://github.com/van-nessing/rat-bar
cd rat-bar
cargo build --release
cp ./target/release/ratbar-providers-rs ~/.config/rat-bar/ratbar-providers-rs
wget https://raw.githubusercontent.com/van-nessing/rat-bar/refs/heads/main/example-config/layout.yaml -P ~/.config/rat-bar
wget https://raw.githubusercontent.com/van-nessing/rat-bar/refs/heads/main/example-config/providers.yaml -P ~/.config/rat-bar
./target/release/ratbar-scripts-rs spawn ./target/release/rat-bar
```

Currently there are 2 files (`providers.yaml` and `layout.yaml`) that need to be present in `~/.config/rat-bar` for rat-bar to start. 
When using the example config you also need to compile the providers package and put the binary into `~/.config/rat-bar`.

The tested way to use rat-bar is with Kitty's [`kitten panel`](https://sw.kovidgoyal.net/kitty/kittens/panel/) and the convenience script `ratbar-scripts-rs spawn` or `scripts.nu spawn all`.

The example scripts can be replaced by anything that periodically outputs json delimited by \n:

```json
{"foo": 1, "bar": "my"}\n
(sleep 1s)
{"foo": 3, "bar": "custom"}\n
(sleep 1s)
{"foo": 5, "bar": "provider"}\n
```

Additional dependencies are `pipewire` for `visualizer`.
Dependencies for the nushell providers are `nvidia-smi` for `nvidia` functionality `playerctl` when using `now-playing` from `scripts.nu`, however all other providers should just work.

<details>

<summary>

## Nix

</summary>

```sh
cd ~/.config/rat-bar
wget https://raw.githubusercontent.com/van-nessing/rat-bar/refs/heads/main/example-config/layout.yaml
wget https://raw.githubusercontent.com/van-nessing/rat-bar/refs/heads/main/example-config/providers.yaml
nix build "github:van-nessing/rat-bar#ratbar-providers-rs"
cp ./result/bin/ratbar-providers-rs ./ratbar-providers-rs
nix run "github:van-nessing/rat-bar#ratbar-scripts-rs" -- spawn
```

### Using with Home Manager

The repo also includes a home module which allows you to configure your bar declaratively and start it automatically.

#### Default setup

```nix
# flake.nix
{
  inputs = {
    rat-bar.url = "github:van-nessing/rat-bar";
  }
}
```

```nix
# home.nix
{
  imports = [ inputs.rat-bar.homeModules.default ];
  # Include to resize with `ratbar-scripts-rs resize`
  home.packages = with pkgs; [ ratbar-scripts-rs ];

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
       providers = lib.getExe pkgs.ratbar-providers-rs;
     in
     {      
       cpu.command = [
         providers
         "cpu"
         "1sec"
         "" # Insert temperature sensor name (nu -c 'sys temp')
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

# Providers

The `Provider` component uses variables supplied by the specified `provider` to display styled text, graphs, bars and images. The `provider` field decides which provider in `providers.yaml` to get its variables from.

`providers.yaml` maps the provider name used in `layout.yaml` to a command that will get executed when the bar starts up

```yaml
clock:
  command:
    - nu
    - ~/.config/rat-bar/provider.nu
    - clock
    - 1sec
cpu:
  command:
    - ratbar-providers-rs
    - cpu
    - 1sec
    - 'k10temp Tccd1'
```
