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
    mut acc: list = 0..<$acc_count | each { 0.0 };
    mut prev = do $stat | each { 0 }
    loop {
        let time = date now;
        let now = do $stat
        let delta = $prev | zip $now | each { $in.0 - $in.1 }
        let total = $delta | math sum
        let load = (1 - ($delta.3 / $total)) * 100 | math round -p 1
        let freq = sys cpu | get freq | math avg | $in / 1000 | math round -p 1  
        let temp = sys temp | where unit == $temp_sensor | first | get temp | into int
        $prev = $now
        $acc = $acc | append $load | skip
        let out = {
            load: $load
            freq: $freq
            temp: $temp
            acc: $acc
        };
        $out | to json -r | print 
        let delta = (date now) - $time
        sleep ($interval - $delta)
    }
}
# Get gpu information
#     variables = []
def "main nvidia" [
    interval: duration # Interval at which messages get sent
    acc_count: int # Number of datapoints for accumulated data
]: [] {
    let memory_q = [used free total] | each { "memory." ++ $in }
    let util_q  = ["utilization.gpu"] 
    let temp_q = ["temperature.gpu"]
    let power_q = ["power.draw.instant"]
    let queries = $util_q ++ $temp_q ++ $power_q ++ $memory_q | str join ,
    mut acc = 0..<$acc_count | each { 0 };

    let stat = {
        nvidia-smi --query-gpu=$"($queries)" --format=csv
        | from csv --trim all
        | rename --block { split row ' ' | first }
        | update cells -c $memory_q { str replace ' ' '' | into filesize | into int | $in / (1GB | into int) | math round -p 1 } 
        | update cells -c $util_q { split words | first | into float | math round -p 1 }
        | update cells -c $power_q { split words | first | into int }
        | first
    }


    loop {
        let time = date now
        let $out = do $stat
        let percent = ($out."memory.used" / $out."memory.total") * 100

        $acc = $acc | append $out."utilization.gpu" | skip 

        $out| insert acc $acc | insert percent $percent | to json -r | print

        let delta = (date now) - $time
        sleep ($interval - $delta)
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
        let time = date now
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
        let delta = (date now) - $time
        sleep ($interval - $delta)
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
        let time = date now;
        let charge = do $charge 
        let status = open $"/sys/class/power_supply/($device)/status";
        let out = {
            charge: $charge
            status: $status
        }
        $out | to json -r | print -r
        let delta = (date now) - $time
        sleep ($interval - $delta)
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
        let delta = (date now) - $now
        sleep ($interval - $delta)
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
