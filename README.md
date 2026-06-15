<img width="1120" height="480" alt="rat-bar-thumbnail" src="https://github.com/user-attachments/assets/1fcfeba8-3630-4ecb-bd06-f6ee7ecefd84" />

# RAT-BAR

A terminal based status bar with mouse interactivity, built in rust using [ratatui](https://ratatui.rs/). It provides various built in widgets such as music player status, an audio visualizer and much more. Custom widgets can also display info provided by any program which can output json formatted lines.

# Quickstart

```sh
git clone https://github.com/van-nessing/rat-bar
cd rat-bar
cargo build --release

mkdir -f ~/.config/rat-bar
cp example-config/layout.yaml ~/.config/rat-bar/layout.yaml
cp example-config/providers.yaml ~/.config/rat-bar/providers.yaml
cp ./target/release/ratbar-providers-rs ~/.config/rat-bar/ratbar-providers-rs

./target/release/ratbar-scripts-rs spawn ./target/release/rat-bar
```

Currently there are 2 files (`providers.yaml` and `layout.kdl`) that need to be present in `~/.config/rat-bar` for rat-bar to start. 
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
Dependencies for the nushell providers are `nvidia-smi` for `nvidia` functionality, `playerctl` when using `now-playing` from `scripts.nu`, and `wpctl` for `pipewire`. However all other providers should just work.

<details>

<summary>

## Nix

</summary>

```sh
cd ~/.config/rat-bar
wget https://raw.githubusercontent.com/van-nessing/rat-bar/refs/heads/main/example-config/layout.yaml
wget https://raw.githubusercontent.com/van-nessing/rat-bar/refs/heads/main/example-config/providers.yaml
nix build "github:van-nessing/rat-bar#ratbar-providers-rs"
# probably not a good idea, collect-garbage will probably fuck up dependencies?
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
  nixpkgs.overlays = [
    inputs.rat-bar.overlays.default
  ];
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
      inherit (inputs.rat-bar.lib)
        bar-element
        group
        layout
        block
        text
        bar
        graph
        image
        ;
    in
    [
      (bar-element { width = "65"; } [
        (block { title = "MEDIA"; })
        # layout for height 1
        # the height argument isn't used for choosing which layout to pick
        # imo it makes the config more readable
        # but it is valid to leave it away
        (layout 1 [
          (group "h" [
            (text "\${media.title} | $[ul](\${media.album}) - $[ul](\${media.artist})")
            (group "h" { width = "8"; } [
              (text "⏮ " { on-click = "media.prev"; })
              (text "\${media.button_symbol} " { on-click = "media.play"; })
              (text "⏭ " { on-click = "media.next"; })
            ])
            (text "\${media.position}/\${media.length}")
          ])
        ])
        # layout for height 2
        (layout 2 [
          
        ])
      ])
      # ... rest of your layout
    ];
  }
}
```

</details>

# Layout

```kdl
// commented parts are optional and often have defaults

// valid color formats for fg and bg are:

// hex (case insensitive)
// #000000

// ansi (case insensitive):
// https://docs.rs/ratatui/0.30.0/ratatui/prelude/enum.Color.html
bar-element /* width="1#" fg="..." bg="..." */ {
  block /* title="my title" fg="..." bg="..." padding=1 */ {
    // borders are enabled by default
    // borders {
    //  left; right; top; bottom;
    // }
  }
  layout /* 1 */ {
    // valid directions are case insensitive: "h", "horizontal", "v", "vertical"
    group "h" /* width="100%" flex="SpaceBetween" center=true spacing=1 */ {
      // width is set to text length by default
      text "some text" /* width="..." on-click="..." on-scroll="..." */
      bar "v" var="provider.variable" fg="..." bg="..." /* width="..." on-click="..." on-scroll="..." */
      graph var="provider.varialbe" fg="..." /* width="..." marker="Octant" fill=true  on-click="..." on-scroll="..."*/
      image "provider.variable" /* width="..." bg="..." on-click="..." on-scroll="..." */
    }
  }
}
```

# Providers

The `Provider` component uses variables supplied by the specified `provider` to display styled text, graphs, bars and images. The `provider` field decides which provider in `providers.yaml` to get its variables from.

`providers.yaml` maps the provider name used in `layout.kdl` to a command that will get executed when the bar starts up

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
