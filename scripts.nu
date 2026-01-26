#!/usr/bin/env nu
def "main spawn all" [lines: int = 4, --bin(-b): string = "rat-bar"] {
  kitten panel --output-name=listjson | from json | get name | each {
    |screen|
    job spawn { main spawn on $screen $lines --bin $bin}
  };
  ^sleep infinity;
}

def "main spawn on" [screen: string, lines: int, --bin(-b): string = "rat-bar"] {
    kitten panel ...[
      --edge=top
      --output-name=$"($screen)"
      --lines=$"($lines)"
      --listen-on unix:/tmp/$"rat-bar-($screen)"
      -o window_padding_width=0
      -o allow_remote_control=yes
      $bin
    ]
}

def "main resize" [lines: int, ...screens: string] {
  $screens | default -e (main get screens) | each { resize $lines $in };
}

def "main get screens" [] {
  kitten panel --output-name=listjson | from json | get name
}

def "resize" [lines: int, screen: string] {
  kitten @ --to unix:/tmp/rat-bar-$"($screen)" resize-os-window --action=os-panel lines=$"($lines)";
}

def "main spawn-sh" [lines: int = 4, --bin (-b): string = "rat-bar"] {
  let cmd = { |screen|  $"kitten panel --edge=top --output-name=($screen) --lines=($lines) --listen-on unix:/tmp/rat-bar-($screen) -o window_padding_width=0 -o allow_remote_control=yes  ($bin)" };
  main get screens | each { do $cmd $in } | prepend "sleep infinity" | str join " & " | bash -c $in
  ^sleep infinity;
}

def "main kill all" [] {
   ^pkill "rat-bar"
}

def main [] {}

