#!/usr/bin/env nu

# Get cpu information
#     variables: [load freq temp acc]
def "main cpu" [
    interval: duration # Interval at which messages get sent
    temp_sensor: string # Name of cpu temperature sensor
    acc_count: int # Number of datapoints for accumulated data
]: [] {
    let stat = {
        open /proc/stat
            | lines
            | first
            | split words
            | skip
            | each { into int }
    }
    mut accumulated: list = 0..$acc_count | each { 0.0 };
    mut prev = do $stat | each { 0 }
    loop {
        let now = do $stat
        let delta = $prev | zip $now | each { $in.0 - $in.1 }
        let total = $delta | math sum
        let load = (1 - ($delta.3 / $total)) * 100 | math round -p 1
        let freq = sys cpu | get freq | math avg | $in / 1000 | math round -p 1  
        let temp = sys temp | where unit == $temp_sensor | first | get temp
        $prev = $now
        $accumulated = $accumulated | append $load | skip
        let out = {
            load: $load
            freq: $freq
            temp: $temp
            acc: $accumulated
        };
        $out | to json -r | print 
        sleep $interval
    }
}

# Get mem information
#     variables [total free used available, ...]
def "main mem" [
    interval: duration # Interval at which messages get sent
]: [] {
    loop {
        sys mem
        | items { |k, v|
            let v = $v
                | into int
                | into float
                | $in / (1GB | into int)
                | math round -p 1
            { $k: $v }
        }
        | into record
        | insert percent (($in.used / $in.total) * 100)
        | to json -r
        | print -r
        sleep $interval
    }
}

# Get net information
#     variables [sent recv]
def "main net" [
    interval: duration # Interval at which messages get sent
    device: string # Net device to use
]: [] {
    let net = { sys net | where name == $device | reject name ip mac | into record }
    mut prev = do $net
    loop {
        let now = do $net
        let tmp = $prev
        let secs = ($interval | into int) / (1sec | into int)
        let out = $tmp | items { |k, prev|
            let now = $now | get $k
            let delta = ($now - $prev) / $secs | into int | $in / (1mb | into int) | math round -p 1
            { $k: $delta }
        } | into record

        $prev = $now
        $out | to json -r | print -r
        sleep $interval
    }
}

# Get battery information
#     variables: [charge]
def "main battery" [
    interval: duration # Interval at which messages get sent
    device: string # Battery device to use
]: [] {
     let charge = {
        let full: int = open $"/sys/class/power_supply/($device)/charge_full" | into int
        let now: int = open $"/sys/class/power_supply/($device)/charge_now" | into int
        ($now / $full) * 100 | into int | append 100 | math min
    };
    loop {
        let charge = do $charge 
        let status = open $"/sys/class/power_supply/($device)/status";
        let out = {
            charge: $charge
            status: $status
        }
        $out | to json -r | print -r
        sleep $interval
    }
}

# Get time information
#     variables: [day time data ...]
def "main clock" [
    interval: duration # Interval at which messages get sent
]: [] {
    loop {
        let now = date now;
        let out = {
            day:  ($now | format date "%a")
            time: ($now | format date "%R")
            date: ($now | format date "%d.%m.%Y")
        };
        $out | to json -r | print -r
        sleep $interval
    }
}

# Get niri event stream
#     variables: [title app_id id pid workspace_id ...]
def "main niri-focus" []: [] {
    niri msg --json focused-window | print;

    niri msg --json event-stream
    | lines
    | each { from json | $in.WindowFocusChanged? }
    | compact
    | each {
        niri msg --json focused-window
        | from json
        | default { title: "", app_id: "", id: "", pid: "", workspace_id: "" }
        | to json -r
        | print -r ;
    }
}

# Various providers that periodically output variables as json
#
# You must use one of the following subcommands. Using this command as-is will only produce this help message. 
def main [] {
  help main
}
