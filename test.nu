lines
| each { from json }
| tee {
  job spawn {
    loop {
      sleep 2sec;
      ^wpctl get-volume @DEFAULT_AUDIO_SINK@
      | parse -r 'Volume: (?<volume>\S*)(?: \[(?<muted>MUTED)\])?'
      | update muted { into bool --relaxed }
      | update volume { into float | $in * 100.0}
      | { update: $in }
      | job send 0;
    }
  };
}
| interleave { 0.. | each { job recv } }
| generate { |message, state = {volume: 0.0, muted: false}|
  mut state = $state

  if $message.update? != null {
    $state = $message.update | first
  }

  if $message.add_vol? != null {
    $state.volume += $message.add_vol;
    let state = $state
    job spawn { ^wpctl set-volume @DEFAULT_AUDIO_SINK@ ($state.volume / 100.0) }
  }

  if $message.toggle_mute? != null {
    $state.muted = not $state.muted
    let state = $state
    job spawn { ^wpctl set-mute @DEFAULT_AUDIO_SINK@ ($state.muted | into int) }
  }

  {out: $state, next: $state}
}
| each { update muted { if $in {"mut"} else {"matl"}} | to json -r | print -r }
