#!/usr/bin/env nu

def "main spawn all" [
    lines: int = 4,
    --bin(-b): string = "rat-bar",
] {
    let screens = main get screens

    let first =  $screens | first
    let rest = $screens | skip

    $"rat-bar spawning on ($screens)" | print

    let pids = $rest| each { |screen|
        job spawn { main spawn on $screen --lines $lines --bin [$bin] }
    };

    let kill_jobs = { job list | get id | each { job kill $in }}

    let log_file = mktemp -t rat-bar.XXX --suffix txt

    "rat-bar spawned" | print

    "rat-bar running" | save -f $log_file

    try { main spawn on $first --lines $lines --bin [nu -c $"^($bin) e> ($log_file)"] e>> $log_file } catch { do $kill_jobs }

    "error encountered, exiting" | print

    open $log_file
}

def "main spawn on" [screen: string, --lines: int = 4, --bin(-b): list<string> = ["rat-bar"]] {
    ^kitten panel ...[
      --edge=top
      --output-name=($screen)
      --lines=($lines)
      --listen-on unix:/tmp/rat-bar-($screen)
      -o window_padding_width=0
      -o allow_remote_control=yes
      ...$bin
    ]
}

def "main resize" [lines: int, ...screens: string] {
    $screens | default -e (main get screens) | each { resize $lines $in };
}

def "main get screens" [] {
    ^kitten panel --output-name=listjson | from json | get name
}

def "resize" [lines: int, screen: string] {
    ^kitten @ --to unix:/tmp/rat-bar-($screen) resize-os-window --action=os-panel lines=($lines);
}

def main [] {
    help main
}

