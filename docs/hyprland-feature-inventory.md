# Hyprland feature inventory — 0.56.2 and current git

This inventory accompanies the implementation plan. All controls are **planned**, not implemented or runtime-tested. It distinguishes source-backed baseline keys, current documentation, and separately versioned components. Defaults below are C++ source expressions, not evaluated runtime values.

Source revision: [efb50993780079460b0cbed1363e2166a2de1d9f](https://github.com/hyprwm/Hyprland/tree/efb50993780079460b0cbed1363e2166a2de1d9f). Current documentation revision: [fde84d41b9a7154a5d211cfb1de50d5910791231](https://github.com/hyprwm/hyprland-wiki/tree/fde84d41b9a7154a5d211cfb1de50d5910791231). Archived wiki: 0.56.0.

## Counts

The baseline is the installed 0.56.2 release. [Current git comparison](#current-git-source-comparison) adds the complete separate 402-key list, 60 window effects, current object/event changes and version differences. The implementation plan explains activation, security and preview requirements.

- 353 unique scalar configuration keys.
- 98 registered Lua API functions, including 51 dispatcher constructors.
- 31 predefined Lua events; custom events additionally supported.
- 34 public animation tree nodes.
- 121 current wiki documents and 100 archived wiki pages inventoried.

Counts describe discovered entries, not an overall completeness percentage. Dynamic plugins and arbitrary Lua programs have no finite universal count.

## Scalar settings

### animations

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [animations:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L264) | Bool | `true` | switch |
| [animations:workspace_wraparound](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L265) | Bool | `false` | switch |

### binds

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [binds:allow_pin_fullscreen](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L539) | Bool | `false` | switch |
| [binds:allow_workspace_cycles](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L529) | Bool | `false` | switch |
| [binds:disable_keybind_grabbing](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L536) | Bool | `false` | switch |
| [binds:drag_threshold](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L540) | Int | `0` | integer control; enumeration picker when a value map exists |
| [binds:focus_preferred_method](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L532) | Int | `0` | integer control; enumeration picker when a value map exists |
| [binds:hide_special_on_workspace_change](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L528) | Bool | `false` | switch |
| [binds:ignore_group_lock](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L533) | Bool | `false` | switch |
| [binds:movefocus_cycles_fullscreen](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L534) | Bool | `false` | switch |
| [binds:movefocus_cycles_groupfirst](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L535) | Bool | `false` | switch |
| [binds:pass_mouse_when_bound](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L525) | Bool | `false` | switch |
| [binds:scroll_event_delay](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L526) | Int | `300` | integer control; enumeration picker when a value map exists |
| [binds:window_direction_monitor_fallback](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L537) | Bool | `true` | switch |
| [binds:workspace_back_and_forth](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L527) | Bool | `false` | switch |
| [binds:workspace_center_on](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L530) | Int | `1` | integer control; enumeration picker when a value map exists |

### cursor

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [cursor:default_monitor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L601) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [cursor:enable_hyprcursor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L606) | Bool | `true` | switch |
| [cursor:hide_on_key_press](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L607) | Bool | `false` | switch |
| [cursor:hide_on_tablet](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L609) | Bool | `false` | switch |
| [cursor:hide_on_touch](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L608) | Bool | `true` | switch |
| [cursor:hotspot_padding](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L593) | Int | `0` | integer control; enumeration picker when a value map exists |
| [cursor:inactive_timeout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L594) | Float | `0` | decimal control |
| [cursor:invisible](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L588) | Bool | `false` | switch |
| [cursor:min_refresh_rate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L592) | Int | `24` | integer control; enumeration picker when a value map exists |
| [cursor:no_break_fs_vrr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L590) | Int | `2` | integer control; enumeration picker when a value map exists |
| [cursor:no_hardware_cursors](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L589) | Int | `2` | integer control; enumeration picker when a value map exists |
| [cursor:no_warps](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L595) | Bool | `false` | switch |
| [cursor:persistent_warps](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L596) | Bool | `false` | switch |
| [cursor:sync_gsettings_theme](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L611) | Bool | `true` | switch |
| [cursor:use_cpu_buffer](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L610) | Int | `2` | integer control; enumeration picker when a value map exists |
| [cursor:warp_back_after_non_mouse_input](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L612) | Bool | `false` | switch |
| [cursor:warp_on_change_workspace](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L597) | Int | `0` | integer control; enumeration picker when a value map exists |
| [cursor:warp_on_toggle_special](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L599) | Int | `0` | integer control; enumeration picker when a value map exists |
| [cursor:zoom_detached_camera](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L605) | Bool | `true` | switch |
| [cursor:zoom_disable_aa](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L604) | Bool | `false` | switch |
| [cursor:zoom_factor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L602) | Float | `1` | decimal control |
| [cursor:zoom_rigid](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L603) | Bool | `false` | switch |

### debug

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [debug:colored_stdout_logs](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L638) | Bool | `true` | switch |
| [debug:damage_blink](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L627) | Bool | `false` | switch |
| [debug:damage_tracking](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L631) | Int | `2` | integer control; enumeration picker when a value map exists |
| [debug:disable_logs](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L629) | Bool | `true` | switch |
| [debug:disable_scale_checks](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L635) | Bool | `false` | switch |
| [debug:disable_time](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L630) | Bool | `true` | switch |
| [debug:ds_handle_same_buffer](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L642) | Bool | `true` | switch |
| [debug:ds_handle_same_buffer_fifo](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L643) | Bool | `true` | switch |
| [debug:enable_stdout_logs](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L632) | Bool | `false` | switch |
| [debug:error_limit](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L636) | Int | `5` | integer control; enumeration picker when a value map exists |
| [debug:error_position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L637) | Int | `0` | integer control; enumeration picker when a value map exists |
| [debug:fifo_pending_workaround](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L644) | Bool | `false` | switch |
| [debug:full_cm_proto](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L641) | Bool | `false` | switch |
| [debug:gl_debugging](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L628) | Bool | `false` | switch |
| [debug:invalidate_fp16](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L647) | Int | `1` | integer control; enumeration picker when a value map exists |
| [debug:log_damage](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L639) | Bool | `false` | switch |
| [debug:manual_crash](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L633) | Int | `0` | integer control; enumeration picker when a value map exists |
| [debug:overlay](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L626) | Bool | `false` | switch |
| [debug:pass](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L640) | Bool | `false` | switch |
| [debug:render_solitary_wo_damage](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L645) | Bool | `false` | switch |
| [debug:suppress_errors](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L634) | Bool | `false` | switch |
| [debug:vfr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L646) | Bool | `true` | switch |

### decoration

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [decoration:active_opacity](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L204) | Float | `1` | decimal control |
| [decoration:blur:brightness](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L246) | Float | `1` | decimal control |
| [decoration:blur:contrast](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L245) | Float | `0.8916` | decimal control |
| [decoration:blur:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L238) | Bool | `true` | switch |
| [decoration:blur:ignore_opacity](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L241) | Bool | `true` | switch |
| [decoration:blur:input_methods](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L253) | Bool | `false` | switch |
| [decoration:blur:input_methods_ignorealpha](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L254) | Float | `0.2` | decimal control |
| [decoration:blur:new_optimizations](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L242) | Bool | `true` | switch |
| [decoration:blur:noise](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L244) | Float | `0.0117` | decimal control |
| [decoration:blur:passes](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L240) | Int | `1` | integer control; enumeration picker when a value map exists |
| [decoration:blur:popups](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L250) | Bool | `false` | switch |
| [decoration:blur:popups_ignorealpha](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L251) | Float | `0.2` | decimal control |
| [decoration:blur:size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L239) | Int | `8` | integer control; enumeration picker when a value map exists |
| [decoration:blur:special](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L249) | Bool | `false` | switch |
| [decoration:blur:vibrancy](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L247) | Float | `0.1696` | decimal control |
| [decoration:blur:vibrancy_darkness](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L248) | Float | `0` | decimal control |
| [decoration:blur:xray](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L243) | Bool | `false` | switch |
| [decoration:border_part_of_window](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L232) | Bool | `true` | switch |
| [decoration:dim_around](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L230) | Float | `0.4` | decimal control |
| [decoration:dim_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L226) | Bool | `false` | switch |
| [decoration:dim_modal](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L225) | Bool | `true` | switch |
| [decoration:dim_special](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L228) | Float | `0.2` | decimal control |
| [decoration:dim_strength](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L227) | Float | `0.5` | decimal control |
| [decoration:fullscreen_opacity](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L206) | Float | `1` | decimal control |
| [decoration:glow:color](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L222) | Gradient | `CHyprColor{0xee33ccff}` | gradient stop/angle editor |
| [decoration:glow:color_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L223) | Gradient | `-1` | gradient stop/angle editor |
| [decoration:glow:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L218) | Bool | `false` | switch |
| [decoration:glow:range](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L219) | Int | `10` | integer control; enumeration picker when a value map exists |
| [decoration:glow:render_power](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L220) | Int | `3` | integer control; enumeration picker when a value map exists |
| [decoration:inactive_opacity](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L205) | Float | `1` | decimal control |
| [decoration:motion_blur:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L257) | Bool | `false` | switch |
| [decoration:motion_blur:samples](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L258) | Int | `7` | integer control; enumeration picker when a value map exists |
| [decoration:rounding](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L200) | Int | `0` | integer control; enumeration picker when a value map exists |
| [decoration:rounding_power](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L202) | Float | `2` | decimal control |
| [decoration:screen_shader](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L231) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [decoration:shadow:color](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L212) | Gradient | `CHyprColor{0xee1a1a1a}` | gradient stop/angle editor |
| [decoration:shadow:color_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L213) | Gradient | `-1` | gradient stop/angle editor |
| [decoration:shadow:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L207) | Bool | `true` | switch |
| [decoration:shadow:offset](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L215) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [decoration:shadow:range](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L208) | Int | `4` | integer control; enumeration picker when a value map exists |
| [decoration:shadow:render_power](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L209) | Int | `3` | integer control; enumeration picker when a value map exists |
| [decoration:shadow:scale](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L217) | Float | `1` | decimal control |
| [decoration:shadow:sharp](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L211) | Bool | `false` | switch |

### dwindle

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [dwindle:default_split_ratio](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L670) | Float | `1` | decimal control |
| [dwindle:force_split](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L662) | Int | `0` | integer control; enumeration picker when a value map exists |
| [dwindle:permanent_direction_override](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L666) | Bool | `false` | switch |
| [dwindle:precise_mouse_move](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L672) | Bool | `false` | switch |
| [dwindle:preserve_split](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L663) | Bool | `false` | switch |
| [dwindle:smart_resizing](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L665) | Bool | `true` | switch |
| [dwindle:smart_split](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L664) | Bool | `false` | switch |
| [dwindle:special_scale_factor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L667) | Float | `1` | decimal control |
| [dwindle:split_bias](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L671) | Int | `0` | integer control; enumeration picker when a value map exists |
| [dwindle:split_width_multiplier](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L668) | Float | `1` | decimal control |
| [dwindle:use_active_for_splits](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L669) | Bool | `true` | switch |

### ecosystem

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [ecosystem:enforce_permissions](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L620) | Bool | `false` | switch |
| [ecosystem:no_donation_nag](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L619) | Bool | `false` | switch |
| [ecosystem:no_update_news](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L618) | Bool | `false` | switch |

### experimental

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [experimental:wp_cm_1_2](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L717) | Bool | `true` | switch |

### general

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [general:allow_tearing](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L185) | Bool | `false` | switch |
| [general:border_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L168) | Int | `1` | integer control; enumeration picker when a value map exists |
| [general:col.active_border](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L174) | Gradient | `CHyprColor{0xffffffff}` | gradient stop/angle editor |
| [general:col.inactive_border](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L173) | Gradient | `CHyprColor{0xff444444}` | gradient stop/angle editor |
| [general:col.nogroup_border](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L175) | Gradient | `CHyprColor{0xffffaaff}` | gradient stop/angle editor |
| [general:col.nogroup_border_active](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L177) | Gradient | `CHyprColor{0xffff00ff}` | gradient stop/angle editor |
| [general:extend_border_grab_area](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L182) | Int | `15` | integer control; enumeration picker when a value map exists |
| [general:float_gaps](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L171) | CssGap | `0` | four-side spacing control |
| [general:gaps_in](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L169) | CssGap | `5` | four-side spacing control |
| [general:gaps_out](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L170) | CssGap | `20` | four-side spacing control |
| [general:gaps_workspaces](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L172) | Int | `0` | integer control; enumeration picker when a value map exists |
| [general:hover_icon_on_border](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L184) | Bool | `true` | switch |
| [general:layout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L179) | String | `"dwindle"` | validated text/picker; dedicated builder for specialized syntax |
| [general:locale](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L194) | String | `""` | validated text/picker; dedicated builder for specialized syntax |
| [general:modal_parent_blocking](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L193) | Bool | `true` | switch |
| [general:no_focus_fallback](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L180) | Bool | `false` | switch |
| [general:resize_corner](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L186) | Int | `0` | integer control; enumeration picker when a value map exists |
| [general:resize_on_border](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L181) | Bool | `false` | switch |
| [general:snap:border_overlap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L191) | Bool | `false` | switch |
| [general:snap:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L188) | Bool | `false` | switch |
| [general:snap:monitor_gap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L190) | Int | `10` | integer control; enumeration picker when a value map exists |
| [general:snap:respect_gaps](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L192) | Bool | `false` | switch |
| [general:snap:window_gap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L189) | Int | `10` | integer control; enumeration picker when a value map exists |

### gestures

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [gestures:close_max_timeout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L411) | Int | `1000` | integer control; enumeration picker when a value map exists |
| [gestures:scrolling:move_snap_cursor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L413) | Bool | `true` | switch |
| [gestures:scrolling:move_snap_to_grid](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L412) | Bool | `true` | switch |
| [gestures:workspace_swipe_cancel_ratio](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L405) | Float | `0.5` | decimal control |
| [gestures:workspace_swipe_create_new](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L406) | Bool | `true` | switch |
| [gestures:workspace_swipe_direction_lock](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L407) | Bool | `true` | switch |
| [gestures:workspace_swipe_direction_lock_threshold](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L408) | Int | `10` | integer control; enumeration picker when a value map exists |
| [gestures:workspace_swipe_distance](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L400) | Int | `300` | integer control; enumeration picker when a value map exists |
| [gestures:workspace_swipe_forever](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L409) | Bool | `false` | switch |
| [gestures:workspace_swipe_invert](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L402) | Bool | `true` | switch |
| [gestures:workspace_swipe_min_speed_to_force](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L404) | Int | `30` | integer control; enumeration picker when a value map exists |
| [gestures:workspace_swipe_touch](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L401) | Bool | `false` | switch |
| [gestures:workspace_swipe_touch_invert](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L403) | Bool | `false` | switch |
| [gestures:workspace_swipe_use_r](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L410) | Bool | `false` | switch |

### group

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [group:auto_group](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L426) | Bool | `true` | switch |
| [group:col.border_active](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L422) | Gradient | `CHyprColor{0x66ffff00}` | gradient stop/angle editor |
| [group:col.border_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L423) | Gradient | `CHyprColor{0x66777700}` | gradient stop/angle editor |
| [group:col.border_locked_active](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L425) | Gradient | `CHyprColor{0x66775500}` | gradient stop/angle editor |
| [group:col.border_locked_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L424) | Gradient | `CHyprColor{0x66ff5500}` | gradient stop/angle editor |
| [group:drag_into_group](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L427) | Int | `1` | integer control; enumeration picker when a value map exists |
| [group:focus_removed_window](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L419) | Bool | `true` | switch |
| [group:group_on_movetoworkspace](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L430) | Bool | `false` | switch |
| [group:groupbar:blur](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L471) | Bool | `false` | switch |
| [group:groupbar:col.active](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L462) | Gradient | `0x66ffff00` | gradient stop/angle editor |
| [group:groupbar:col.inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L463) | Gradient | `0x66777700` | gradient stop/angle editor |
| [group:groupbar:col.locked_active](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L464) | Gradient | `0x66ff5500` | gradient stop/angle editor |
| [group:groupbar:col.locked_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L465) | Gradient | `0x66775500` | gradient stop/angle editor |
| [group:groupbar:disable_when_only](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L437) | Bool | `false` | switch |
| [group:groupbar:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L436) | Bool | `true` | switch |
| [group:groupbar:font_family](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L439) | String | `"[[EMPTY]]"` | validated text/picker; dedicated builder for specialized syntax |
| [group:groupbar:font_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L442) | Int | `8` | integer control; enumeration picker when a value map exists |
| [group:groupbar:font_weight_active](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L440) | FontWeight | `400 /* FontWeight constructor default */` | weight selector |
| [group:groupbar:font_weight_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L441) | FontWeight | `400 /* FontWeight constructor default */` | weight selector |
| [group:groupbar:gaps_in](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L467) | Int | `2` | integer control; enumeration picker when a value map exists |
| [group:groupbar:gaps_out](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L466) | Int | `2` | integer control; enumeration picker when a value map exists |
| [group:groupbar:gradient_round_only_edges](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L457) | Bool | `true` | switch |
| [group:groupbar:gradient_rounding](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L454) | Int | `2` | integer control; enumeration picker when a value map exists |
| [group:groupbar:gradient_rounding_power](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L455) | Float | `2` | decimal control |
| [group:groupbar:gradients](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L443) | Bool | `false` | switch |
| [group:groupbar:height](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L444) | Int | `14` | integer control; enumeration picker when a value map exists |
| [group:groupbar:indicator_gap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L445) | Int | `0` | integer control; enumeration picker when a value map exists |
| [group:groupbar:indicator_height](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L446) | Int | `3` | integer control; enumeration picker when a value map exists |
| [group:groupbar:keep_upper_gap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L468) | Bool | `true` | switch |
| [group:groupbar:middle_click_close](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L451) | Bool | `true` | switch |
| [group:groupbar:priority](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L448) | Int | `3` | integer control; enumeration picker when a value map exists |
| [group:groupbar:render_titles](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L449) | Bool | `true` | switch |
| [group:groupbar:round_only_edges](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L456) | Bool | `true` | switch |
| [group:groupbar:rounding](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L452) | Int | `1` | integer control; enumeration picker when a value map exists |
| [group:groupbar:rounding_power](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L453) | Float | `2` | decimal control |
| [group:groupbar:scrolling](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L450) | Bool | `true` | switch |
| [group:groupbar:stacked](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L447) | Bool | `false` | switch |
| [group:groupbar:text_color](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L458) | Color | `0xffffffff` | color picker |
| [group:groupbar:text_color_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L459) | Color | `-1` | color picker |
| [group:groupbar:text_color_locked_active](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L460) | Color | `-1` | color picker |
| [group:groupbar:text_color_locked_inactive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L461) | Color | `-1` | color picker |
| [group:groupbar:text_offset](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L469) | Int | `0` | integer control; enumeration picker when a value map exists |
| [group:groupbar:text_padding](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L470) | Int | `0` | integer control; enumeration picker when a value map exists |
| [group:insert_after_current](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L418) | Bool | `true` | switch |
| [group:merge_floated_into_tiled_on_groupbar](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L429) | Bool | `false` | switch |
| [group:merge_groups_on_drag](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L420) | Bool | `true` | switch |
| [group:merge_groups_on_groupbar](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L421) | Bool | `true` | switch |

### input

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [input:accel_profile](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L283) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:emulate_discrete_scroll](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L311) | Int | `1` | integer control; enumeration picker when a value map exists |
| [input:float_switch_override_focus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L304) | Int | `1` | integer control; enumeration picker when a value map exists |
| [input:focus_on_close](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L301) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:follow_mouse](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L298) | Int | `1` | integer control; enumeration picker when a value map exists |
| [input:follow_mouse_shrink](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L313) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:follow_mouse_threshold](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L300) | Float | `0` | decimal control |
| [input:force_no_accel](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L285) | Bool | `false` | switch |
| [input:kb_file](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L276) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_layout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L272) | String | `"us"` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_model](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L271) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_options](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L274) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_rules](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L275) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_variant](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L273) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:left_handed](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L288) | Bool | `false` | switch |
| [input:mouse_refocus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L303) | Bool | `true` | switch |
| [input:natural_scroll](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L297) | Bool | `false` | switch |
| [input:numlock_by_default](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L277) | Bool | `false` | switch |
| [input:off_window_axis_events](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L309) | Int | `1` | integer control; enumeration picker when a value map exists |
| [input:repeat_delay](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L280) | Int | `600` | integer control; enumeration picker when a value map exists |
| [input:repeat_rate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L279) | Int | `25` | integer control; enumeration picker when a value map exists |
| [input:resolve_binds_by_sym](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L278) | Bool | `false` | switch |
| [input:rotation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L286) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:scroll_button](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L293) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:scroll_button_lock](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L294) | Bool | `false` | switch |
| [input:scroll_factor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L296) | Float | `1` | decimal control |
| [input:scroll_method](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L291) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:scroll_points](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L289) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:sensitivity](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L281) | Float | `0` | decimal control |
| [input:special_fallthrough](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L308) | Bool | `false` | switch |
| [input:tablet:absolute_region_position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L366) | Bool | `false` | switch |
| [input:tablet:active_area_position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L374) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [input:tablet:active_area_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L372) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [input:tablet:left_handed](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L371) | Bool | `false` | switch |
| [input:tablet:output](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L363) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:tablet:region_position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L364) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [input:tablet:region_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L368) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [input:tablet:relative_input](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L370) | Bool | `false` | switch |
| [input:tablet:transform](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L362) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:tablettool:eraser_button_mode](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L381) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:tablettool:eraser_button_override](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L385) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:tablettool:pressure_range_max](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L392) | Float | `-1.0` | decimal control |
| [input:tablettool:pressure_range_min](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L389) | Float | `-1.0` | decimal control |
| [input:touchdevice:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L347) | Bool | `true` | switch |
| [input:touchdevice:output](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L346) | String | `"[[Auto]]"` | validated text/picker; dedicated builder for specialized syntax |
| [input:touchdevice:transform](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L345) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:touchpad:clickfinger_behavior](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L329) | Bool | `false` | switch |
| [input:touchpad:disable_while_typing](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L322) | Bool | `true` | switch |
| [input:touchpad:drag_3fg](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L338) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:touchpad:drag_lock](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L333) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:touchpad:flip_x](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L336) | Bool | `false` | switch |
| [input:touchpad:flip_y](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L337) | Bool | `false` | switch |
| [input:touchpad:middle_button_emulation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L325) | Bool | `false` | switch |
| [input:touchpad:natural_scroll](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L323) | Bool | `false` | switch |
| [input:touchpad:scroll_factor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L324) | Float | `1` | decimal control |
| [input:touchpad:tap-and-drag](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L335) | Bool | `true` | switch |
| [input:touchpad:tap-to-click](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L331) | Bool | `true` | switch |
| [input:touchpad:tap_button_map](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L327) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:virtualkeyboard:release_pressed_on_close](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L355) | Bool | `false` | switch |
| [input:virtualkeyboard:share_states](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L353) | Int | `2` | integer control; enumeration picker when a value map exists |

### input-capture

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [input-capture:capture_modifiers](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L722) | Bool | `false` | switch |
| [input-capture:enforce_barriers](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L723) | Bool | `true` | switch |

### layout

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [layout:single_window_aspect_ratio](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L653) | Vec2 | `Config::VEC2{0, 0}` | paired coordinate controls |
| [layout:single_window_aspect_ratio_tolerance](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L655) | Float | `0.1F` | decimal control |

### master

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [master:allow_small_split](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L678) | Bool | `false` | switch |
| [master:always_keep_position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L692) | Bool | `false` | switch |
| [master:center_ignores_reserved](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L689) | Bool | `false` | switch |
| [master:center_master_fallback](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L687) | String | `"left"` | validated text/picker; dedicated builder for specialized syntax |
| [master:drop_at_cursor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L691) | Bool | `true` | switch |
| [master:focus_master_on_close](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L694) | Bool | `false` | switch |
| [master:mfact](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L680) | Float | `0.55` | decimal control |
| [master:new_on_active](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L683) | String | `"none"` | validated text/picker; dedicated builder for specialized syntax |
| [master:new_on_top](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L682) | Bool | `false` | switch |
| [master:new_status](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L681) | String | `"slave"` | validated text/picker; dedicated builder for specialized syntax |
| [master:orientation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L684) | String | `"left"` | validated text/picker; dedicated builder for specialized syntax |
| [master:slave_count_for_center_master](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L685) | Int | `2` | integer control; enumeration picker when a value map exists |
| [master:smart_resizing](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L690) | Bool | `true` | switch |
| [master:special_scale_factor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L679) | Float | `1` | decimal control |

### misc

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [misc:allow_session_lock_restore](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L498) | Bool | `false` | switch |
| [misc:always_follow_on_dnd](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L488) | Bool | `true` | switch |
| [misc:animate_manual_resizes](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L490) | Bool | `false` | switch |
| [misc:animate_mouse_windowdragging](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L491) | Bool | `false` | switch |
| [misc:anr_missed_pings](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L516) | Int | `5` | integer control; enumeration picker when a value map exists |
| [misc:background_color](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L501) | Color | `0xff111111` | color picker |
| [misc:close_special_on_empty](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L502) | Bool | `true` | switch |
| [misc:col.splash](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L479) | Color | `0x55ffffff` | color picker |
| [misc:disable_autoreload](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L492) | Bool | `false` | switch |
| [misc:disable_hyprland_guiutils_check](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L512) | Bool | `false` | switch |
| [misc:disable_hyprland_logo](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L477) | Bool | `false` | switch |
| [misc:disable_scale_notification](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L518) | Bool | `false` | switch |
| [misc:disable_splash_rendering](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L478) | Bool | `false` | switch |
| [misc:disable_watchdog_warning](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L513) | Bool | `false` | switch |
| [misc:disable_xdg_env_checks](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L511) | Bool | `false` | switch |
| [misc:enable_anr_dialog](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L515) | Bool | `true` | switch |
| [misc:enable_swallow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L493) | Bool | `false` | switch |
| [misc:exit_window_retains_fullscreen](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L505) | Bool | `false` | switch |
| [misc:focus_on_activate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L496) | Bool | `false` | switch |
| [misc:font_family](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L480) | String | `"Sans"` | validated text/picker; dedicated builder for specialized syntax |
| [misc:force_default_wallpaper](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L482) | Int | `-1` | integer control; enumeration picker when a value map exists |
| [misc:initial_workspace_token_timeout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L507) | Int | `10` | integer control; enumeration picker when a value map exists |
| [misc:initial_workspace_tracking](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L506) | Int | `1` | integer control; enumeration picker when a value map exists |
| [misc:key_press_enables_dpms](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L486) | Bool | `false` | switch |
| [misc:layers_hog_keyboard_focus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L489) | Bool | `true` | switch |
| [misc:lockdead_screen_delay](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L514) | Int | `1000` | integer control; enumeration picker when a value map exists |
| [misc:middle_click_paste](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L509) | Bool | `true` | switch |
| [misc:mouse_move_enables_dpms](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L485) | Bool | `false` | switch |
| [misc:mouse_move_focuses_monitor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L497) | Bool | `true` | switch |
| [misc:name_vk_after_proc](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L487) | Bool | `true` | switch |
| [misc:on_focus_under_fullscreen](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L503) | Int | `2` | integer control; enumeration picker when a value map exists |
| [misc:render_unfocused_fps](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L510) | Int | `15` | integer control; enumeration picker when a value map exists |
| [misc:screencopy_force_8b](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L517) | Bool | `true` | switch |
| [misc:session_lock_blur](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L500) | Bool | `false` | switch |
| [misc:session_lock_xray](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L499) | Bool | `false` | switch |
| [misc:size_limits_tiled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L519) | Bool | `false` | switch |
| [misc:splash_font_family](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L481) | String | `"[[EMPTY]]"` | validated text/picker; dedicated builder for specialized syntax |
| [misc:swallow_exception_regex](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L495) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [misc:swallow_regex](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L494) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [misc:vrr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L483) | Int | `0` | integer control; enumeration picker when a value map exists |

### opengl

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [opengl:nvidia_anti_flicker](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L556) | Bool | `true` | switch |

### quirks

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [quirks:prefer_hdr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L729) | Int | `0` | integer control; enumeration picker when a value map exists |
| [quirks:skip_non_kms_dmabuf_formats](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L730) | Bool | `false` | switch |

### render

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [render:cm_auto_hdr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L569) | Int | `1` | integer control; enumeration picker when a value map exists |
| [render:cm_enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L567) | Bool | `true` | switch |
| [render:cm_sdr_eotf](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L573) | String | `"default"` | validated text/picker; dedicated builder for specialized syntax |
| [render:commit_timing_enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L574) | Bool | `true` | switch |
| [render:ctm_animation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L565) | Int | `2` | integer control; enumeration picker when a value map exists |
| [render:direct_scanout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L562) | Int | `0` | integer control; enumeration picker when a value map exists |
| [render:expand_undersized_textures](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L563) | Bool | `true` | switch |
| [render:fp16_sdr_tf](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L582) | Int | `0` | integer control; enumeration picker when a value map exists |
| [render:icc_vcgt_enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L575) | Bool | `true` | switch |
| [render:keep_unmodified_copy](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L578) | Int | `2` | integer control; enumeration picker when a value map exists |
| [render:new_render_scheduling](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L571) | Bool | `false` | switch |
| [render:non_shader_cm](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L572) | Int | `3` | integer control; enumeration picker when a value map exists |
| [render:non_shader_cm_interop](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L580) | Int | `2` | integer control; enumeration picker when a value map exists |
| [render:send_content_type](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L568) | Bool | `true` | switch |
| [render:use_fp16](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L577) | Int | `2` | integer control; enumeration picker when a value map exists |
| [render:use_shader_blur_blend](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L576) | Bool | `false` | switch |
| [render:xp_mode](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L564) | Bool | `false` | switch |

### scrolling

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [scrolling:column_width](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L702) | Float | `0.5` | decimal control |
| [scrolling:direction](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L709) | String | `"right"` | validated text/picker; dedicated builder for specialized syntax |
| [scrolling:explicit_column_widths](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L708) | String | `"0.333, 0.5, 0.667, 1.0"` | validated text/picker; dedicated builder for specialized syntax |
| [scrolling:focus_fit_method](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L703) | Int | `1` | integer control; enumeration picker when a value map exists |
| [scrolling:follow_focus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L705) | Bool | `true` | switch |
| [scrolling:follow_min_visible](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L706) | Float | `0.4` | decimal control |
| [scrolling:fullscreen_on_one_column](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L700) | Bool | `true` | switch |
| [scrolling:wrap_focus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L710) | Bool | `true` | switch |
| [scrolling:wrap_swapcol](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L711) | Bool | `true` | switch |

### xwayland

| Source key | Type | Default expression | Planned control |
|---|---|---|---|
| [xwayland:create_abstract_socket](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L550) | Bool | `false` | switch |
| [xwayland:enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L547) | Bool | `true` | switch |
| [xwayland:force_zero_scaling](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L549) | Bool | `false` | switch |
| [xwayland:use_nearest_neighbor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L548) | Bool | `true` | switch |

## Structured configuration fields

Monitor additionally requires `output`; device requires `name`; workspace rules require `workspace` and may use `enabled` and `layout_opts`; window/layer rules support `name`, `enabled` and `match`. These wrapper fields are additional to the descriptor lists below.

### monitor — 22 descriptor fields

| Field | Parser type | Constructor/default expression |
|---|---|---|
| [mode](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L82) | String | `"preferred"` |
| [position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L84) | String | `"auto"` |
| [scale](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L86) | String | `"auto"` |
| [reserved](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L88) | CssGap | `0` |
| [reserved_area](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L93) | CssGap | `0` |
| [disabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L98) | Bool | `false` |
| [transform](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L103) | Int | `0, 0, 7` |
| [mirror](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L108) | String | `STRVAL_EMPTY` |
| [bitdepth](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L113) | Int | `8` |
| [cm](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L118) | String | `"srgb"` |
| [sdr_eotf](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L120) | String | `"default"` |
| [sdrbrightness](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L125) | Float | `1.F` |
| [sdrsaturation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L130) | Float | `1.F` |
| [vrr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L135) | Int | `-1, -1, 3` |
| [icc](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L141) | String | `STRVAL_EMPTY` |
| [supports_wide_color](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L143) | Int | `0, -1, 1` |
| [supports_hdr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L148) | Int | `0, -1, 1` |
| [sdr_min_luminance](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L153) | Float | `0.2F` |
| [sdr_max_luminance](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L158) | Int | `80` |
| [min_luminance](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L163) | Float | `-1.F` |
| [max_luminance](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L168) | Int | `-1` |
| [max_avg_luminance](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L173) | Int | `-1` |

### device — 43 descriptor fields

| Field | Parser type | Constructor/default expression |
|---|---|---|
| [sensitivity](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L231) | Float | `0.F, -1.F, 1.F` |
| [accel_profile](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L232) | String | `STRVAL_EMPTY` |
| [rotation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L233) | Int | `0, 0, 359` |
| [kb_file](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L234) | String | `STRVAL_EMPTY` |
| [kb_layout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L235) | String | `"us"` |
| [kb_variant](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L236) | String | `STRVAL_EMPTY` |
| [kb_options](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L237) | String | `STRVAL_EMPTY` |
| [kb_rules](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L238) | String | `STRVAL_EMPTY` |
| [kb_model](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L239) | String | `STRVAL_EMPTY` |
| [repeat_rate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L240) | Int | `25, 0, 200` |
| [repeat_delay](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L241) | Int | `600, 0, 2000` |
| [natural_scroll](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L242) | Bool | `false` |
| [tap_button_map](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L243) | String | `STRVAL_EMPTY` |
| [numlock_by_default](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L244) | Bool | `false` |
| [resolve_binds_by_sym](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L245) | Bool | `false` |
| [disable_while_typing](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L246) | Bool | `true` |
| [clickfinger_behavior](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L247) | Bool | `false` |
| [middle_button_emulation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L248) | Bool | `false` |
| [tap_to_click](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L249) | Bool | `true` |
| [tap_and_drag](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L250) | Bool | `true` |
| [drag_lock](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L251) | Int | `0, 0, 2` |
| [left_handed](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L252) | Bool | `false` |
| [scroll_method](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L253) | String | `STRVAL_EMPTY` |
| [scroll_button](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L254) | Int | `0, 0, 300` |
| [scroll_button_lock](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L255) | Bool | `false` |
| [scroll_points](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L256) | String | `STRVAL_EMPTY` |
| [scroll_factor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L257) | Float | `1.F, 0.F, 100.F` |
| [transform](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L258) | Int | `-1` |
| [output](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L259) | String | `STRVAL_EMPTY` |
| [enabled](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L260) | Bool | `true` |
| [region_position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L261) | Vec2 | `{0, 0}` |
| [absolute_region_position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L262) | Bool | `false` |
| [region_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L263) | Vec2 | `{0, 0}` |
| [relative_input](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L264) | Bool | `false` |
| [active_area_position](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L265) | Vec2 | `{0, 0}` |
| [active_area_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L266) | Vec2 | `{0, 0}` |
| [flip_x](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L267) | Bool | `false` |
| [flip_y](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L268) | Bool | `false` |
| [drag_3fg](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L269) | Int | `0, 0, 2` |
| [keybinds](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L270) | Bool | `true` |
| [share_states](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L271) | Int | `0, 0, 2` |
| [release_pressed_on_close](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L272) | Bool | `false` |
| [tags](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L273) | String | `STRVAL_EMPTY` |

### workspace_rule — 15 descriptor fields

| Field | Parser type | Constructor/default expression |
|---|---|---|
| [monitor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L198) | String | `STRVAL_EMPTY` |
| [default](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L200) | Bool | `false` |
| [persistent](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L202) | Bool | `false` |
| [gaps_in](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L204) | CssGap | `5` |
| [gaps_out](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L206) | CssGap | `20` |
| [float_gaps](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L208) | CssGap | `0` |
| [border_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L210) | Int | `-1` |
| [no_border](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L212) | Bool | `false` |
| [no_rounding](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L214) | Bool | `false` |
| [decorate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L216) | Bool | `true` |
| [no_shadow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L218) | Bool | `false` |
| [on_created_empty](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L220) | String | `STRVAL_EMPTY` |
| [default_name](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L222) | String | `STRVAL_EMPTY` |
| [layout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L224) | String | `STRVAL_EMPTY` |
| [animation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L226) | String | `STRVAL_EMPTY` |

### window_rule_effect — 57 descriptor fields

| Field | Parser type | Constructor/default expression |
|---|---|---|
| [float](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L50) | Bool | `false` |
| [tile](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L51) | Bool | `false` |
| [fullscreen](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L52) | Bool | `false` |
| [maximize](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L53) | Bool | `false` |
| [center](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L54) | Bool | `false` |
| [pseudo](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L55) | Bool | `false` |
| [no_initial_focus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L56) | Bool | `false` |
| [pin](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L57) | Bool | `false` |
| [fullscreen_state](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L58) | String | `STRVAL_EMPTY` |
| [move](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L59) | ExpressionVec2 | `` |
| [size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L60) | ExpressionVec2 | `` |
| [monitor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L61) | String | `STRVAL_EMPTY` |
| [workspace](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L62) | String | `STRVAL_EMPTY` |
| [group](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L63) | String | `STRVAL_EMPTY` |
| [suppress_event](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L64) | String | `STRVAL_EMPTY` |
| [content](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L65) | String | `STRVAL_EMPTY` |
| [no_close_for](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L66) | Int | `0` |
| [scrolling_width](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L67) | Float | `0.F` |
| [rounding](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L68) | Int | `0, 0, 20` |
| [border_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L69) | Int | `0` |
| [rounding_power](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L70) | Float | `2.F, 1.F, 10.F` |
| [scroll_mouse](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L71) | Float | `1.F, 0.01F, 10.F` |
| [scroll_touchpad](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L72) | Float | `1.F, 0.01F, 10.F` |
| [animation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L73) | String | `STRVAL_EMPTY` |
| [idle_inhibit](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L74) | String | `STRVAL_EMPTY` |
| [opacity](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L75) | String | `STRVAL_EMPTY` |
| [tag](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L76) | String | `STRVAL_EMPTY` |
| [max_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L77) | ExpressionVec2 | `` |
| [min_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L78) | ExpressionVec2 | `` |
| [border_color](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L79) | Gradient | `CHyprColor(0xFF000000)` |
| [persistent_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L80) | Bool | `false` |
| [allows_input](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L81) | Bool | `false` |
| [dim_around](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L82) | Bool | `false` |
| [decorate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L83) | Bool | `true` |
| [focus_on_activate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L84) | Bool | `false` |
| [keep_aspect_ratio](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L85) | Bool | `false` |
| [nearest_neighbor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L86) | Bool | `false` |
| [no_anim](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L87) | Bool | `false` |
| [no_blur](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L88) | Bool | `false` |
| [no_dim](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L89) | Bool | `false` |
| [no_focus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L90) | Bool | `false` |
| [no_follow_mouse](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L91) | Bool | `false` |
| [no_max_size](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L92) | Bool | `false` |
| [no_shadow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L93) | Bool | `false` |
| [no_shortcuts_inhibit](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L94) | Bool | `false` |
| [opaque](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L95) | Bool | `false` |
| [force_rgbx](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L96) | Bool | `false` |
| [sync_fullscreen](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L97) | Bool | `false` |
| [immediate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L98) | Bool | `false` |
| [xray](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L99) | Bool | `false` |
| [render_unfocused](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L100) | Bool | `false` |
| [no_screen_share](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L101) | Bool | `false` |
| [no_vrr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L102) | Bool | `false` |
| [no_auto_hdr](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L103) | Bool | `false` |
| [stay_focused](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L104) | Bool | `false` |
| [confine_pointer](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L105) | Bool | `false` |
| [tonemap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L106) | String | `STRVAL_EMPTY` |

### layer_rule_effect — 10 descriptor fields

| Field | Parser type | Constructor/default expression |
|---|---|---|
| [no_anim](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L183) | Bool | `false` |
| [blur](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L184) | Bool | `false` |
| [blur_popups](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L185) | Bool | `false` |
| [ignore_alpha](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L186) | Float | `0.F, 0.F, 1.F` |
| [dim_around](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L187) | Bool | `false` |
| [xray](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L188) | Bool | `false` |
| [animation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L189) | String | `STRVAL_EMPTY` |
| [order](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L190) | Int | `0` |
| [above_lock](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L191) | Int | `0, 0, 2` |
| [no_screen_share](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L192) | Bool | `false` |

### Match properties

Window: `class`, `title`, `initial_class`, `initial_title`, `float`, `tag`, `xwayland`, `fullscreen`, `pin`, `focus`, `group`, `modal`, `fullscreen_state_internal`, `fullscreen_state_client`, `workspace`, `content`, `xdg_tag`.

Layer: `namespace`.

### Binding options

`allow_input_capture`, `auto_consuming`, `click`, `desc`, `description`, `device`, `dont_inhibit`, `drag`, `ignore_mods`, `locked`, `long_press`, `non_consuming`, `release`, `repeating`, `submap_universal`, `transparent`.

The `device` option contains inclusive/exclusive selection and device/tag lists. Metadata stubs alone miss some accepted flags; parser code must be checked.

### Animation tree

| Node | Parent |
|---|---|
| global | root |
| windows | global |
| layers | global |
| fade | global |
| border | global |
| borderangle | global |
| shadowangle | global |
| glowangle | global |
| workspaces | global |
| zoomFactor | global |
| monitorAdded | global |
| layersIn | layers |
| layersOut | layers |
| windowsIn | windows |
| windowsOut | windows |
| windowsMove | windows |
| fadeIn | fade |
| fadeOut | fade |
| fadeSwitch | fade |
| fadeShadow | fade |
| fadeGlow | fade |
| fadeDim | fade |
| fadeLayers | fade |
| fadeLayersIn | fadeLayers |
| fadeLayersOut | fadeLayers |
| fadePopups | fade |
| fadePopupsIn | fadePopups |
| fadePopupsOut | fadePopups |
| fadeDpms | fade |
| workspacesIn | workspaces |
| workspacesOut | workspaces |
| specialWorkspace | workspaces |
| specialWorkspaceIn | specialWorkspace |
| specialWorkspaceOut | specialWorkspace |

## Lua API functions

These need typed action nodes, dedicated editors or read-only inspection. Function presence does not establish argument correctness. The generated upstream stubs are useful discovery aids, not a sufficient schema.

- `hl.animation` — api
- `hl.bind` — api
- `hl.clear_crashed_lockscreen` — api
- `hl.config` — api
- `hl.curve` — api
- `hl.define_submap` — api
- `hl.device` — api
- `hl.dispatch` — api
- `hl.env` — api
- `hl.exec_cmd` — api
- `hl.exec_scheduled_prop_refresh_immediately` — api
- `hl.gesture` — api
- `hl.get_active_monitor` — api
- `hl.get_active_special_workspace` — api
- `hl.get_active_window` — api
- `hl.get_active_workspace` — api
- `hl.get_config` — api
- `hl.get_current_submap` — api
- `hl.get_cursor_pos` — api
- `hl.get_last_window` — api
- `hl.get_last_workspace` — api
- `hl.get_layers` — api
- `hl.get_loaded_plugins` — api
- `hl.get_monitor` — api
- `hl.get_monitor_at` — api
- `hl.get_monitor_at_cursor` — api
- `hl.get_monitors` — api
- `hl.get_urgent_window` — api
- `hl.get_window` — api
- `hl.get_windows` — api
- `hl.get_workspace` — api
- `hl.get_workspace_windows` — api
- `hl.get_workspaces` — api
- `hl.is_key_down` — api
- `hl.layer_rule` — api
- `hl.monitor` — api
- `hl.on` — api
- `hl.permission` — api
- `hl.timer` — api
- `hl.unbind` — api
- `hl.version` — api
- `hl.window_rule` — api
- `hl.workspace_rule` — api
- `hl.dsp.dpms` — dispatcher
- `hl.dsp.event` — dispatcher
- `hl.dsp.exec_cmd` — dispatcher
- `hl.dsp.exec_raw` — dispatcher
- `hl.dsp.exit` — dispatcher
- `hl.dsp.focus` — dispatcher
- `hl.dsp.force_idle` — dispatcher
- `hl.dsp.force_renderer_reload` — dispatcher
- `hl.dsp.global` — dispatcher
- `hl.dsp.layout` — dispatcher
- `hl.dsp.no_op` — dispatcher
- `hl.dsp.pass` — dispatcher
- `hl.dsp.release_input_capture` — dispatcher
- `hl.dsp.send_key_state` — dispatcher
- `hl.dsp.send_shortcut` — dispatcher
- `hl.dsp.submap` — dispatcher
- `hl.dsp.cursor.move` — dispatcher
- `hl.dsp.cursor.move_to_corner` — dispatcher
- `hl.dsp.group.active` — dispatcher
- `hl.dsp.group.lock` — dispatcher
- `hl.dsp.group.lock_active` — dispatcher
- `hl.dsp.group.move_window` — dispatcher
- `hl.dsp.group.next` — dispatcher
- `hl.dsp.group.prev` — dispatcher
- `hl.dsp.group.toggle` — dispatcher
- `hl.dsp.window.alter_zorder` — dispatcher
- `hl.dsp.window.bring_to_top` — dispatcher
- `hl.dsp.window.center` — dispatcher
- `hl.dsp.window.clear_tags` — dispatcher
- `hl.dsp.window.close` — dispatcher
- `hl.dsp.window.cycle_next` — dispatcher
- `hl.dsp.window.deny_from_group` — dispatcher
- `hl.dsp.window.drag` — dispatcher
- `hl.dsp.window.float` — dispatcher
- `hl.dsp.window.fullscreen` — dispatcher
- `hl.dsp.window.fullscreen_state` — dispatcher
- `hl.dsp.window.kill` — dispatcher
- `hl.dsp.window.move` — dispatcher
- `hl.dsp.window.pin` — dispatcher
- `hl.dsp.window.pseudo` — dispatcher
- `hl.dsp.window.resize` — dispatcher
- `hl.dsp.window.set_prop` — dispatcher
- `hl.dsp.window.signal` — dispatcher
- `hl.dsp.window.swap` — dispatcher
- `hl.dsp.window.tag` — dispatcher
- `hl.dsp.window.toggle_swallow` — dispatcher
- `hl.dsp.workspace.change_id` — dispatcher
- `hl.dsp.workspace.move` — dispatcher
- `hl.dsp.workspace.rename` — dispatcher
- `hl.dsp.workspace.swap_monitors` — dispatcher
- `hl.dsp.workspace.toggle_special` — dispatcher
- `hl.layout.register` — api
- `hl.notification.create` — api
- `hl.notification.get` — api
- `hl.plugin.load` — api

## Lua events

`config.props_refreshed`, `config.reloaded`, `hyprland.shutdown`, `hyprland.start`, `input.keyboard.key`, `keybinds.submap`, `layer.closed`, `layer.opened`, `monitor.added`, `monitor.focused`, `monitor.layout_changed`, `monitor.removed`, `screenshare.state`, `window.active`, `window.class`, `window.close`, `window.destroy`, `window.fullscreen`, `window.kill`, `window.move_to_workspace`, `window.open`, `window.open_early`, `window.pin`, `window.title`, `window.update_rules`, `window.urgent`, `workspace.active`, `workspace.created`, `workspace.move_to_monitor`, `workspace.removed`, `workspace.special_active`.

Custom events must be represented separately; payloads require explicit typed contracts.

## Runtime object fields and methods

Source-derived inference below identifies inspection and action surfaces. Read-only fields must not be presented as mutable settings. Setter/method acceptance needs explicit tests.

### HL.EventSubscription

Methods: `is_active`, `remove`.

Fields: none discovered.

### HL.Group

Methods: `add`, `remove`.

Fields: `current` (HL.Window\|nil), `current_index` (integer), `denied` (boolean), `locked` (boolean), `members` (HL.Window\|table\|nil), `size` (integer).

### HL.Keybind

Methods: `is_enabled`, `remove`, `set_enabled`, `unbind`.

Fields: `allow_input_capture` (boolean\|nil), `arg` (string), `auto_consuming` (boolean), `catchall` (boolean), `click` (boolean), `description` (any), `device_inclusive` (boolean), `devices` (any), `display_key` (string), `dont_inhibit` (boolean), `drag` (boolean), `enabled` (boolean), `handler` (string), `has_description` (boolean), `ignore_mods` (boolean), `key` (string), `keycode` (integer), `locked` (boolean), `long_press` (boolean), `modmask` (integer), `mouse` (boolean), `non_consuming` (boolean), `release` (boolean), `repeating` (boolean), `submap` (string), `submap_universal` (boolean), `transparent` (boolean).

### HL.LayerRule

Methods: `is_enabled`, `set_enabled`.

Fields: none discovered.

### HL.LayerSurface

Methods: none discovered.

Fields: `above_fullscreen` (boolean\|nil), `address` (string), `h` (integer), `interactivity` (integer), `layer` (integer), `mapped` (boolean), `monitor` (HL.Monitor\|nil), `namespace` (string), `pid` (integer), `w` (integer), `x` (integer), `y` (integer).

### HL.Monitor

Methods: `set_special_workspace`, `set_workspace`.

Fields: `active_special_workspace` (HL.Workspace\|nil), `active_workspace` (HL.Workspace\|nil), `available_modes` (boolean\|integer\|number\|table), `cm` (string), `description` (string), `dpms_status` (boolean), `focused` (boolean), `height` (integer), `id` (integer), `is_mirror` (boolean), `mirrors` (HL.Monitor\|table), `name` (string), `physical_height` (integer), `physical_width` (integer), `position` (integer\|table), `refresh_rate` (number), `reserved` (number\|table), `scale` (number), `serial` (string), `size` (integer\|table), `transform` (integer), `vrr_active` (boolean), `width` (integer), `x` (integer), `y` (integer).

### HL.Notification

Methods: `dismiss`, `get_color`, `get_elapsed`, `get_elapsed_since_creation`, `get_font_size`, `get_icon`, `get_text`, `get_timeout`, `is_alive`, `is_paused`, `pause`, `resume`, `set_color`, `set_font_size`, `set_icon`, `set_paused`, `set_text`, `set_timeout`.

Fields: none discovered.

### HL.Timer

Methods: `is_enabled`, `set_enabled`, `set_timeout`.

Fields: none discovered.

### HL.Window

Methods: none discovered.

Fields: `accepts_input` (boolean), `active` (boolean), `address` (string), `allowed_over_fullscreen` (boolean), `at` (integer\|table), `class` (string), `content_type` (string), `floating` (boolean), `focus_history_id` (integer), `fullscreen` (integer), `fullscreen_client` (integer), `fullscreen_handler` (string), `group` (HL.Group\|nil), `hidden` (boolean), `inhibiting_idle` (boolean), `initial_class` (string), `initial_title` (string), `layout` (HL.Window\|boolean\|integer\|number\|string\|table\|nil), `mapped` (boolean), `monitor` (HL.Monitor\|nil), `pid` (integer), `pin_fullscreened` (boolean), `pinned` (boolean), `size` (integer\|table), `stable_id` (integer), `swallowing` (HL.Window\|nil), `tags` (string\|table), `tearing_hint` (boolean\|nil), `title` (string), `visible` (boolean), `workspace` (HL.Workspace\|nil), `xdg_description` (string\|nil), `xdg_tag` (string\|nil), `xwayland` (boolean).

### HL.WindowRule

Methods: `is_enabled`, `set_enabled`.

Fields: none discovered.

### HL.Workspace

Methods: `get_groups`, `get_windows`.

Fields: `active` (boolean), `config_name` (string), `fullscreen_mode` (integer), `fullscreen_window` (HL.Window\|nil), `groups` (integer\|nil), `has_fullscreen` (boolean), `has_urgent` (boolean), `id` (integer), `is_empty` (boolean), `is_persistent` (boolean), `last_window` (HL.Window\|nil), `monitor` (HL.Monitor\|nil), `name` (string), `special` (boolean), `tiled_layout` (string), `visible` (boolean), `windows` (integer).

### HL.WorkspaceRule

Methods: `is_enabled`, `set_enabled`.

Fields: none discovered.

## Other typed specifications

Values below are inferred from the generated stubs and must be cross-checked before implementation. Spring/animation and binding discrepancies are discussed in the plan.

### HL.LayoutTarget

| Name | Inferred type |
|---|---|
| index | integer |
| window | HL.Window\|nil |
| box | HL.Box |
| place | fun(self: HL.LayoutTarget, box: HL.Box): nil |
| set_box | fun(self: HL.LayoutTarget, box: HL.Box): nil |

### HL.LayoutContext

| Name | Inferred type |
|---|---|
| area | HL.Box |
| targets | HL.LayoutTarget[] |
| grid_cell | fun(self: HL.LayoutContext, i: integer, cols: integer, rows?: integer): HL.Box |
| column | fun(self: HL.LayoutContext, i: integer, n: integer): HL.Box |
| row | fun(self: HL.LayoutContext, i: integer, n: integer): HL.Box |
| split | fun(self: HL.LayoutContext, box: HL.Box, side: 'left'\|'right'\|'top'\|'bottom'\|'up'\|'down', ratio: number): HL.Box |

### HL.LayoutProvider

| Name | Inferred type |
|---|---|
| recalculate | fun(ctx: HL.LayoutContext): nil |
| layout_msg? | fun(ctx: HL.LayoutContext, msg: string): boolean\|string\|nil |

### HL.BindOptions

| Name | Inferred type |
|---|---|
| repeating? | boolean |
| locked? | boolean |
| release? | boolean |
| non_consuming? | boolean |
| transparent? | boolean |
| ignore_mods? | boolean |
| dont_inhibit? | boolean |
| long_press? | boolean |
| submap_universal? | boolean |
| click? | boolean |
| drag? | boolean |
| description? | string |
| desc? | string |
| device? | {inclusive?: boolean, list?: string[]} |
| allow_input_capture? | boolean |

### HL.TimerOptions

| Name | Inferred type |
|---|---|
| timeout | integer |
| type | "repeat"\|"oneshot" |

### HL.GestureSpec

| Name | Inferred type |
|---|---|
| fingers | integer |
| direction | string |
| action | string\|function |
| mods? | string |
| scale? | number |
| mode? | string |
| zoom_level? | number |
| workspace_name? | string |
| disable_inhibit? | boolean |

### HL.PermissionSpec

| Name | Inferred type |
|---|---|
| binary | string |
| type | string |
| mode | string |

### HL.NotificationOptions

| Name | Inferred type |
|---|---|
| text | string |
| timeout | number |
| color? | string |
| icon? | integer\|string |
| font_size? | number |

### HL.LayerQueryFilter

| Name | Inferred type |
|---|---|
| monitor? | HL.MonitorSelector |
| namespace? | string |

### HL.WindowQueryFilter

| Name | Inferred type |
|---|---|
| class? | string |
| floating? | boolean |
| mapped? | boolean |
| monitor? | HL.MonitorSelector |
| tag? | string |
| title? | string |
| workspace? | HL.WorkspaceSelector |

## Layout message inventory

These names come from the current docs and require 0.56.2 handler checks before use. See the plan for version gating.

- **dwindle-layout**: `movetoroot`, `preselect`, `rotatesplit`, `splitratio`, `swapsplit`, `togglesplit`
- **master-layout**: `addmaster`, `cyclenext`, `cycleprev`, `focusmaster`, `mfact`, `orientationbottom`, `orientationcenter`, `orientationcycle`, `orientationleft`, `orientationnext`, `orientationprev`, `orientationright`, `orientationtop`, `removemaster`, `rollnext`, `rollprev`, `swapnext`, `swapprev`, `swapwithmaster`
- **monocle-layout**: `cyclenext`, `cycleprev`
- **scrolling-layout**: `center`, `colresize`, `consume`, `consume_or_expel`, `expel`, `fit`, `fit_into_view`, `focus`, `inhibit_scroll`, `move`, `promote`, `swapcol`

## Hyprctl command registry

`workspaces`, `workspacerules`, `activeworkspace`, `clients`, `kill`, `activewindow`, `layers`, `version`, `devices`, `splash`, `cursorpos`, `binds`, `globalshortcuts`, `systeminfo`, `animations`, `rollinglog`, `configerrors`, `locked`, `descriptions`, `submap`, `status`, `reloadshaders`, `monitors`, `reload`, `plugin`, `notify`, `dismissnotify`, `getprop`, `seterror`, `switchxkblayout`, `output`, `dispatch`, `keyword`, `setcursor`, `getoption`, `decorations`, `[[BATCH]]`, `eval`, `repl`.

`instances` and certain daemon-specific commands are handled by the CLI or other processes rather than this compositor registry. Request type does not imply read-only behavior; for example `kill` mutates the session.

## Separately versioned ecosystem settings

All documented table-entry names are included below; not every entry is a setting (some are CLI switches/widget types). An installed-component version and its own source schema are required before claiming adapter completeness. No original prose descriptions are copied.

### hypridle

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| General | [lock_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| General | [unlock_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| General | [on_lock_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| General | [on_unlock_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| General | [before_sleep_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| General | [after_sleep_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| General | [ignore_dbus_inhibit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | bool | false |
| General | [ignore_systemd_inhibit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | bool | false |
| General | [ignore_wayland_inhibit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | bool | false |
| General | [inhibit_sleep](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | int | 2 |
| Listeners | [timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | int | none, value must be specified |
| Listeners | [on-timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| Listeners | [on-resume](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| Listeners | [ignore_inhibit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | bool | false |
| Listeners | [condition_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | string | [[Empty]] |
| Listeners | [condition_retry](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | int | 0 |

### hyprland-qt-support

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Configuration | [roundness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprland-qt-support.md) | int | 1 |
| Configuration | [border_width](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprland-qt-support.md) | int | 1 |
| Configuration | [reduce_motion](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprland-qt-support.md) | bool | false |

### hyprlauncher

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| General | [grab_focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | bool | true |
| Cache | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | bool | true |
| Finders | [default_finder](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | string | desktop |
| Finders | [desktop_prefix](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | string | [[Empty]] |
| Finders | [unicode_prefix](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | string | . |
| Finders | [math_prefix](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | string | = |
| Finders | [font_prefix](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | string | ' |
| Finders | [desktop_launch_prefix](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | string | [[Empty]] |
| Finders | [desktop_icons](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | bool | true |
| UI | [window_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | vec2 | 400 260 |

### hyprlock

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Command-line Arguments | [-v`/`--verbose](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | -v`/`--verbose |  |
| Command-line Arguments | [-q`/`--quiet](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | -q`/`--quiet |  |
| Command-line Arguments | [-c`/`--config` FILE](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | -c`/`--config` FILE |  |
| Command-line Arguments | [--display` NAME](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | --display` NAME |  |
| Command-line Arguments | [--grace` SECONDS](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | --grace` SECONDS |  |
| Command-line Arguments | [--immediate-render](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | --immediate-render |  |
| Command-line Arguments | [--no-fade-in](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | --no-fade-in |  |
| Command-line Arguments | [-V`/`--version](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | -V`/`--version |  |
| Command-line Arguments | [-h`/`--help](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | -h`/`--help |  |
| Variable Types | [layoutxy](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | layoutxy |  |
| General | [hide_cursor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | false |
| General | [ignore_empty_input](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | false |
| General | [immediate_render](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | false |
| General | [text_trim](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | true |
| General | [fractional_scaling](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 2 |
| General | [screencopy_mode](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| General | [fail_timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 2000 |
| Authentication | [pam:enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | true |
| Authentication | [pam:module](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | hyprlock |
| Authentication | [fingerprint:enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | false |
| Authentication | [fingerprint:ready_message](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | (Scan fingerprint to unlock) |
| Authentication | [fingerprint:present_message](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | Scanning fingerprint |
| Authentication | [fingerprint:retry_delay](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 250 |
| Animations | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | true |
| Keyboard Shortcuts and Actions | [ESC](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) |  |  |
| Keyboard Shortcuts and Actions | [Ctrl + u](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) |  |  |
| Keyboard Shortcuts and Actions | [Ctrl + Backspace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) |  |  |
| Shadowable | [shadow_passes](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Shadowable | [shadow_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 3 |
| Shadowable | [shadow_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | color | rgb(0,0,0) |
| Shadowable | [shadow_boost](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | 1.2 |
| Clickable | [onclick](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Background | [monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Background | [path](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Background | [color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | color | rgba(17, 17, 17, 1.0) |
| Background | [blur_passes](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Background | [blur_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 7 |
| Background | [noise](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | 0.0117 |
| Background | [contrast](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | 0.8916 |
| Background | [brightness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | 0.8172 |
| Background | [vibrancy](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | 0.1696 |
| Background | [vibrancy_darkness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | 0.05 |
| Background | [reload_time](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | -1 |
| Background | [reload_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Background | [crossfade_time](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | -1.0 |
| Background | [zindex](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | -1 |
| Image | [monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Image | [path](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Image | [size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 150 |
| Image | [rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | -1 |
| Image | [border_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 4 |
| Image | [border_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | gradient | rgba(221, 221, 221, 1.0) |
| Image | [rotate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Image | [reload_time](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | -1 |
| Image | [reload_cmd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Image | [position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | layoutxy | 0, 0 |
| Image | [halign](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |
| Image | [valign](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |
| Image | [zindex](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Shape | [monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Shape | [size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | layoutxy | 100, 100 |
| Shape | [color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | color | rgba(17, 17, 17, 1.0) |
| Shape | [rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | -1 |
| Shape | [rotate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Shape | [border_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Shape | [border_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | gradient | rgba(0, 207, 230, 1.0) |
| Shape | [xray](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | false |
| Shape | [position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | layoutxy | 0, 0 |
| Shape | [halign](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |
| Shape | [valign](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |
| Shape | [zindex](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Input Field | [monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Input Field | [size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | layoutxy | 400, 90 |
| Input Field | [outline_thickness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 4 |
| Input Field | [dots_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | 0.25 |
| Input Field | [dots_spacing](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | float | 0.15 |
| Input Field | [dots_center](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | true |
| Input Field | [dots_rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | -1 |
| Input Field | [dots_text_format](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Input Field | [outer_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | gradient | rgba(17, 17, 17, 1.0) |
| Input Field | [inner_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | color | rgba(200, 200, 200, 1.0) |
| Input Field | [font_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | color | rgba(10, 10, 10, 1.0) |
| Input Field | [font_family](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | Noto Sans |
| Input Field | [fade_on_empty](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | true |
| Input Field | [fade_timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 2000 |
| Input Field | [placeholder_text](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | <i>Input Password...</i> |
| Input Field | [hide_input](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | false |
| Input Field | [hide_input_base_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | color | rgba(153, 170, 187) |
| Input Field | [rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | -1 |
| Input Field | [check_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | gradient | rgba(204, 136, 34, 1.0) |
| Input Field | [check_text](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Input Field | [fail_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | gradient | rgba(204, 34, 34, 1.0) |
| Input Field | [fail_text](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | <i>$FAIL <b>($ATTEMPTS)</b></i> |
| Input Field | [capslock_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | gradient | [[Empty]] |
| Input Field | [numlock_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | gradient | [[Empty]] |
| Input Field | [bothlock_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | gradient | [[Empty]] |
| Input Field | [invert_numlock](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | false |
| Input Field | [swap_font_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | bool | false |
| Input Field | [position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | layoutxy | 0, 0 |
| Input Field | [halign](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |
| Input Field | [valign](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |
| Input Field | [zindex](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Label | [monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | [[Empty]] |
| Label | [text](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | Sample Text |
| Label | [text_align](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |
| Label | [color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | color | rgba(254, 254, 254, 1.0) |
| Label | [font_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 16 |
| Label | [font_family](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | Sans |
| Label | [rotate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | int | 0 |
| Label | [position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | layoutxy | 0, 0 |
| Label | [halign](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |
| Label | [valign](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | string | center |

### hyprpaper

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Setting wallpapers | [monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) |  |  |
| Setting wallpapers | [path](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) |  |  |
| Setting wallpapers | [fit_mode](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) |  |  |
| Setting wallpapers | [timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) |  |  |
| Setting wallpapers | [order](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) |  |  |
| Setting wallpapers | [recursive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) |  |  |
| Misc Options | [splash](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) | bool | true |
| Misc Options | [splash_offset](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) | float | 20 |
| Misc Options | [splash_opacity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) | float | 0.8 |
| Misc Options | [ipc](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) | bool | true |

### hyprpicker

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Configuration | [-a`/`--autocopy](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-f`/`--format=](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-o`/`--output-format=](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-n`/`--notify](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-b`/`--no-fancy](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-h`/`--help](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-r`/`--render-inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-z`/`--no-zoom](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-q`/`--quiet](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-v`/`--verbose](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-t`/`--no-fractional](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-d`/`--disable-hex-preview](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-l`/`--lowercase-hex](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-s`/`--scale=scale](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-u`/`--radius=radius](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-N`/`--name](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |
| Configuration | [-V`/`--version](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) |  |  |

### hyprqt6engine

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Theme | [color_scheme](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | string | [[Empty]] |
| Theme | [icon_theme](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | string | [[Empty]] |
| Theme | [style](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | string | Fusion |
| Theme | [font_fixed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | string | monospace |
| Theme | [font_fixed_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | int | 11 |
| Theme | [font](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | string | Sans Serif |
| Theme | [font_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | int | 11 |
| Misc | [single_click_activate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | bool | true |
| Misc | [menus_have_icons](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | bool | true |
| Misc | [shortcuts_for_context_menus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | bool | true |

### hyprshutdown

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Command-Line Options | [--vt N](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) |  |  |
| Command-Line Options | [--dry-run](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) |  |  |
| Command-Line Options | [--no-exit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) |  |  |
| Command-Line Options | [--top-label`, `-t](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) |  |  |
| Command-Line Options | [--post-cmd`, `-p](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) |  |  |
| Command-Line Options | [--no-fork](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) |  |  |
| Command-Line Options | [--verbose](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) |  |  |
| Command-Line Options | [--help`, `-h](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) |  |  |

### hyprsunset

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Configuration | [max-gamma](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprsunset.md) | int | 100 |
| Profile | [time](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprsunset.md) | string | 00:00 |
| Profile | [temperature](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprsunset.md) | int | 6000 |
| Profile | [gamma](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprsunset.md) | float | 1.0 |
| Profile | [identity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprsunset.md) | bool | false |
| Profile | [on-switch](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprsunset.md) | string | [[Empty]] |

### hyprtoolkit

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Configuration | [background](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | color | 0xFF181818 |
| Configuration | [base](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | color | 0xFF202020 |
| Configuration | [text](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | color | 0xFFDADADA |
| Configuration | [alternate_base](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | color | 0xFF272727 |
| Configuration | [bright_text](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | color | 0xFFFFDEDE |
| Configuration | [accent](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | color | 0xFF00FFCC |
| Configuration | [accent_secondary](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | color | 0xFF0099F0 |
| Configuration | [h1_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | int | 19 |
| Configuration | [h2_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | int | 15 |
| Configuration | [h3_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | int | 13 |
| Configuration | [font_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | int | 11 |
| Configuration | [small_font_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | int | 10 |
| Configuration | [icon_theme](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | string | [[Empty]] |
| Configuration | [font_family](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | string | Sans Serif |
| Configuration | [font_family_monospace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | string | monospace |
| Configuration | [rounding_large](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | int | 10 |
| Configuration | [rounding_small](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | int | 5 |

### xdg-desktop-portal-hyprland

| Section | Documented name | Type or argument | Default |
|---|---|---|---|
| Screencopy | [max_fps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/xdg-desktop-portal-hyprland.md) | int | 120 |
| Screencopy | [allow_token_by_default](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/xdg-desktop-portal-hyprland.md) | bool | false |
| Screencopy | [custom_picker_binary](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/xdg-desktop-portal-hyprland.md) | string | "hyprland-share-picker" |
| Screencopy | [force_shm](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/xdg-desktop-portal-hyprland.md) | bool | false |
| Screencopy | [cursor_mode](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/xdg-desktop-portal-hyprland.md) | int | 0 |

## Documentation coverage register

Each current wiki page was inventoried and assigned a role. Configuration and safety documentation received focused semantic/source checks; reference/index and external-integration pages were triaged. Linked third-party manuals are not claimed as audited. The machine-readable inventory retains headings, table-entry names and hashes.

| Current document | Planned role |
|---|---|
| [Hyprland Wiki](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/_index.md) | documentation governance/reference; no compositor setting presumed |
| [Community](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/community/_index.md) | documentation governance/reference; no compositor setting presumed |
| [Configuring](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/_index.md) | core settings, structured editors, automation or compatibility |
| [Lua code snippets](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/code-snippets.md) | core settings, structured editors, automation or compatibility |
| [Core](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/_index.md) | core settings, structured editors, automation or compatibility |
| [Advanced configuration](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/_index.md) | core settings, structured editors, automation or compatibility |
| [Lua events](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) | core settings, structured editors, automation or compatibility |
| [Lua utilities](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/lua-utilities.md) | core settings, structured editors, automation or compatibility |
| [Notifications](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) | core settings, structured editors, automation or compatibility |
| [Permissions](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/permissions.md) | core settings, structured editors, automation or compatibility |
| [Using hyprctl](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/using-hyprctl.md) | core settings, structured editors, automation or compatibility |
| [Animations](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/animations.md) | core settings, structured editors, automation or compatibility |
| [Autostart](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/autostart.md) | core settings, structured editors, automation or compatibility |
| [Binds](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/_index.md) | core settings, structured editors, automation or compatibility |
| [Device binds](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/devices/_index.md) | core settings, structured editors, automation or compatibility |
| [Mouse](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/devices/mouse.md) | core settings, structured editors, automation or compatibility |
| [Per-device binds](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/devices/per-device.md) | core settings, structured editors, automation or compatibility |
| [Touchpad](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/devices/touchpad.md) | core settings, structured editors, automation or compatibility |
| [Flags](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) | core settings, structured editors, automation or compatibility |
| [Gestures](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | core settings, structured editors, automation or compatibility |
| [Global hotkeys and binds](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/globals.md) | core settings, structured editors, automation or compatibility |
| [Keyboard layouts](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/keyboard-layouts.md) | core settings, structured editors, automation or compatibility |
| [Submaps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/submaps.md) | core settings, structured editors, automation or compatibility |
| [Switches](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/switches.md) | core settings, structured editors, automation or compatibility |
| [Config options](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | core settings, structured editors, automation or compatibility |
| [Devices](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/devices.md) | core settings, structured editors, automation or compatibility |
| [Dispatchers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) | core settings, structured editors, automation or compatibility |
| [Environment variables](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/environment-variables.md) | core settings, structured editors, automation or compatibility |
| [Monitors](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | core settings, structured editors, automation or compatibility |
| [Colors and colorspaces](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/colors.md) | core settings, structured editors, automation or compatibility |
| [Monitor modes](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/modes.md) | core settings, structured editors, automation or compatibility |
| [Output selection](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/output-selection.md) | core settings, structured editors, automation or compatibility |
| [Positioning](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/positioning.md) | core settings, structured editors, automation or compatibility |
| [Rules](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/_index.md) | core settings, structured editors, automation or compatibility |
| [Layer rules](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | core settings, structured editors, automation or compatibility |
| [Window Rules](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | core settings, structured editors, automation or compatibility |
| [Workspace Rules](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | core settings, structured editors, automation or compatibility |
| [Extra](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/extra/_index.md) | core settings, structured editors, automation or compatibility |
| [Multi-GPU](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/extra/multi-gpu.md) | core settings, structured editors, automation or compatibility |
| [Performance](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/extra/performance.md) | core settings, structured editors, automation or compatibility |
| [Systemd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/extra/systemd.md) | core settings, structured editors, automation or compatibility |
| [Tearing](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/extra/tearing.md) | core settings, structured editors, automation or compatibility |
| [Virtual GPU](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/extra/virtual-gpu.md) | core settings, structured editors, automation or compatibility |
| [Xwayland](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/extra/xwayland.md) | core settings, structured editors, automation or compatibility |
| [Layouts](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/_index.md) | core settings, structured editors, automation or compatibility |
| [Custom layouts](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/custom-layouts.md) | core settings, structured editors, automation or compatibility |
| [Dwindle layout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | core settings, structured editors, automation or compatibility |
| [Master layout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | core settings, structured editors, automation or compatibility |
| [Monocle layout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/monocle-layout.md) | core settings, structured editors, automation or compatibility |
| [Scrolling layout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | core settings, structured editors, automation or compatibility |
| [Naming conventions](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | core settings, structured editors, automation or compatibility |
| [Selectors](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/selectors.md) | core settings, structured editors, automation or compatibility |
| [Contributing and debugging](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/contributing-and-debugging/_index.md) | documentation governance/reference; no compositor setting presumed |
| [Issue guidelines](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/contributing-and-debugging/issue-guidelines.md) | documentation governance/reference; no compositor setting presumed |
| [PR guidelines](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/contributing-and-debugging/pr-guidelines.md) | documentation governance/reference; no compositor setting presumed |
| [Tests](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/contributing-and-debugging/tests.md) | documentation governance/reference; no compositor setting presumed |
| [Translations](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/contributing-and-debugging/translations.md) | documentation governance/reference; no compositor setting presumed |
| [Wiki writing guidelines](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/contributing-and-debugging/wiki-guidelines.md) | documentation governance/reference; no compositor setting presumed |
| [Crashes and Bugs](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/crashes-and-bugs/_index.md) | deployment, generated config ownership or diagnostics |
| [FAQ](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/faq/_index.md) | deployment, generated config ownership or diagnostics |
| [Getting Started](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/getting-started/_index.md) | deployment, generated config ownership or diagnostics |
| [Installation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/getting-started/installation.md) | deployment, generated config ownership or diagnostics |
| [Master tutorial](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/getting-started/master-tutorial.md) | deployment, generated config ownership or diagnostics |
| [Preconfigured setups](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/getting-started/preconfigured-setups.md) | deployment, generated config ownership or diagnostics |
| [Hypr Ecosystem](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/_index.md) | documentation governance/reference; no compositor setting presumed |
| [Developer libs and toolkits](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/dev/_index.md) | library/toolkit compatibility; hyprlang legacy parser support |
| [aquamarine](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/dev/aquamarine.md) | library/toolkit compatibility; hyprlang legacy parser support |
| [hyprgraphics](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/dev/hyprgraphics.md) | library/toolkit compatibility; hyprlang legacy parser support |
| [hyprland-guiutils](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/dev/hyprland-guiutils.md) | library/toolkit compatibility; hyprlang legacy parser support |
| [hyprlang](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/dev/hyprlang.md) | library/toolkit compatibility; hyprlang legacy parser support |
| [hyprtoolkit (developer docs)](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/dev/hyprtoolkit.md) | library/toolkit compatibility; hyprlang legacy parser support |
| [hyprutils](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/dev/hyprutils.md) | library/toolkit compatibility; hyprlang legacy parser support |
| [hyprwayland-scanner](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/dev/hyprwayland-scanner.md) | library/toolkit compatibility; hyprlang legacy parser support |
| [User apps and utilities](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/_index.md) | separately versioned ecosystem adapter |
| [hyprcursor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprcursor.md) | separately versioned ecosystem adapter |
| [hypridle](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hypridle.md) | separately versioned ecosystem adapter |
| [hyprland-qt-support](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprland-qt-support.md) | separately versioned ecosystem adapter |
| [hyprlauncher](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlauncher.md) | separately versioned ecosystem adapter |
| [hyprlock](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprlock.md) | separately versioned ecosystem adapter |
| [hyprpaper](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpaper.md) | separately versioned ecosystem adapter |
| [hyprpicker](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpicker.md) | separately versioned ecosystem adapter |
| [hyprpolkitagent](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpolkitagent.md) | separately versioned ecosystem adapter |
| [hyprpwcenter](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprpwcenter.md) | separately versioned ecosystem adapter |
| [hyprqt6engine](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprqt6engine.md) | separately versioned ecosystem adapter |
| [hyprshutdown](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprshutdown.md) | separately versioned ecosystem adapter |
| [hyprsunset](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprsunset.md) | separately versioned ecosystem adapter |
| [hyprsysteminfo](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprsysteminfo.md) | separately versioned ecosystem adapter |
| [hyprtoolkit (user docs)](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/hyprtoolkit.md) | separately versioned ecosystem adapter |
| [xdg-desktop-portal-hyprland](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hypr-ecosystem/user/xdg-desktop-portal-hyprland.md) | separately versioned ecosystem adapter |
| [Hyprland plugins](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hyprland-plugins/_index.md) | plugin trust, build compatibility and adapter contracts |
| [Development](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hyprland-plugins/development/_index.md) | plugin trust, build compatibility and adapter contracts |
| [Advanced](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hyprland-plugins/development/advanced.md) | plugin trust, build compatibility and adapter contracts |
| [Getting started](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hyprland-plugins/development/getting-started.md) | plugin trust, build compatibility and adapter contracts |
| [Plugin guidelines](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hyprland-plugins/development/plugin-guidelines.md) | plugin trust, build compatibility and adapter contracts |
| [Using plugins](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/hyprland-plugins/using-plugins.md) | plugin trust, build compatibility and adapter contracts |
| [IPC](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/ipc/_index.md) | runtime discovery, events and control transport |
| [Nix](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nix/_index.md) | deployment, generated config ownership or diagnostics |
| [Advanced](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nix/advanced.md) | deployment, generated config ownership or diagnostics |
| [Cachix](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nix/cachix.md) | deployment, generated config ownership or diagnostics |
| [Configuring Hyprland with Hjem](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nix/configuring-hyprland-with-hjem.md) | deployment, generated config ownership or diagnostics |
| [Configuring Hyprland with Home Manager](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nix/configuring-hyprland-with-home-manager.md) | deployment, generated config ownership or diagnostics |
| [Contributing and Debugging](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nix/contributing-and-debugging.md) | deployment, generated config ownership or diagnostics |
| [Hyprland on any distro using Nix](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nix/hyprland-on-any-distro-using-nix.md) | deployment, generated config ownership or diagnostics |
| [Hyprland on NixOS](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nix/installing-hyprland-on-nixos.md) | deployment, generated config ownership or diagnostics |
| [NVIDIA](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/nvidia/_index.md) | deployment, generated config ownership or diagnostics |
| [Useful utilities](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/_index.md) | integration candidates; external manuals are separate scope |
| [App clients](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/app-clients.md) | integration candidates; external manuals are separate scope |
| [App launchers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/app-launchers.md) | integration candidates; external manuals are separate scope |
| [Clipboard managers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/clipboard-managers.md) | integration candidates; external manuals are separate scope |
| [Color pickers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/color-pickers.md) | integration candidates; external manuals are separate scope |
| [File managers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/file-managers.md) | integration candidates; external manuals are separate scope |
| [Hypr ecosystem](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/hypr-ecosystem.md) | integration candidates; external manuals are separate scope |
| [Must-have](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/must-have.md) | integration candidates; external manuals are separate scope |
| [Other](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/other.md) | integration candidates; external manuals are separate scope |
| [Phone connect](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/phone-connect.md) | integration candidates; external manuals are separate scope |
| [Screen sharing](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/screen-sharing.md) | integration candidates; external manuals are separate scope |
| [Screenshots and recording](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/screenshots-and-recording.md) | integration candidates; external manuals are separate scope |
| [Status bars](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/status-bars.md) | integration candidates; external manuals are separate scope |
| [UWSM](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/uwsm.md) | integration candidates; external manuals are separate scope |
| [Wallpapers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/useful-utilities/wallpapers.md) | integration candidates; external manuals are separate scope |
| [Version selector ⚙️](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/version-selector.md) | documentation governance/reference; no compositor setting presumed |

### Archived 0.56 wiki pages

All 100 pages collected without fetch errors. This release-family documentation is checked against the exact 0.56.2 source where behavior differs.

- [](https://wiki.hypr.land/0.56.0/)
- [Configuring](https://wiki.hypr.land/0.56.0/Configuring/)
- [Configuring/Advanced-and-Cool](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/)
- [Configuring/Advanced-and-Cool/Animations](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Animations/)
- [Configuring/Advanced-and-Cool/Devices](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Devices/)
- [Configuring/Advanced-and-Cool/Environment-variables](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Environment-variables/)
- [Configuring/Advanced-and-Cool/Expanding-functionality](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Expanding-functionality/)
- [Configuring/Advanced-and-Cool/Gestures](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Gestures/)
- [Configuring/Advanced-and-Cool/Multi-GPU](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Multi-GPU/)
- [Configuring/Advanced-and-Cool/Notifications](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Notifications/)
- [Configuring/Advanced-and-Cool/Performance](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Performance/)
- [Configuring/Advanced-and-Cool/Permissions](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Permissions/)
- [Configuring/Advanced-and-Cool/Tearing](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Tearing/)
- [Configuring/Advanced-and-Cool/Uncommon-tips-and-tricks](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Uncommon-tips-and-tricks/)
- [Configuring/Advanced-and-Cool/Using-hyprctl](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Using-hyprctl/)
- [Configuring/Advanced-and-Cool/Virtual-GPU](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/Virtual-GPU/)
- [Configuring/Advanced-and-Cool/XWayland](https://wiki.hypr.land/0.56.0/Configuring/Advanced-and-Cool/XWayland/)
- [Configuring/Basics](https://wiki.hypr.land/0.56.0/Configuring/Basics/)
- [Configuring/Basics/Autostart](https://wiki.hypr.land/0.56.0/Configuring/Basics/Autostart/)
- [Configuring/Basics/Binds](https://wiki.hypr.land/0.56.0/Configuring/Basics/Binds/)
- [Configuring/Basics/Dispatchers](https://wiki.hypr.land/0.56.0/Configuring/Basics/Dispatchers/)
- [Configuring/Basics/Monitors](https://wiki.hypr.land/0.56.0/Configuring/Basics/Monitors/)
- [Configuring/Basics/Variables](https://wiki.hypr.land/0.56.0/Configuring/Basics/Variables/)
- [Configuring/Basics/Window-Rules](https://wiki.hypr.land/0.56.0/Configuring/Basics/Window-Rules/)
- [Configuring/Basics/Workspace-Rules](https://wiki.hypr.land/0.56.0/Configuring/Basics/Workspace-Rules/)
- [Configuring/Example-configurations](https://wiki.hypr.land/0.56.0/Configuring/Example-configurations/)
- [Configuring/Layouts](https://wiki.hypr.land/0.56.0/Configuring/Layouts/)
- [Configuring/Layouts/Custom-Layouts](https://wiki.hypr.land/0.56.0/Configuring/Layouts/Custom-Layouts/)
- [Configuring/Layouts/Dwindle-Layout](https://wiki.hypr.land/0.56.0/Configuring/Layouts/Dwindle-Layout/)
- [Configuring/Layouts/Master-Layout](https://wiki.hypr.land/0.56.0/Configuring/Layouts/Master-Layout/)
- [Configuring/Layouts/Monocle-Layout](https://wiki.hypr.land/0.56.0/Configuring/Layouts/Monocle-Layout/)
- [Configuring/Layouts/Scrolling-Layout](https://wiki.hypr.land/0.56.0/Configuring/Layouts/Scrolling-Layout/)
- [Configuring/Start](https://wiki.hypr.land/0.56.0/Configuring/Start/)
- [Connect](https://wiki.hypr.land/0.56.0/Connect/)
- [Contributing-and-Debugging](https://wiki.hypr.land/0.56.0/Contributing-and-Debugging/)
- [Contributing-and-Debugging/Issue-Guidelines](https://wiki.hypr.land/0.56.0/Contributing-and-Debugging/Issue-Guidelines/)
- [Contributing-and-Debugging/PR-Guidelines](https://wiki.hypr.land/0.56.0/Contributing-and-Debugging/PR-Guidelines/)
- [Contributing-and-Debugging/Tests](https://wiki.hypr.land/0.56.0/Contributing-and-Debugging/Tests/)
- [Contributing-and-Debugging/Translations](https://wiki.hypr.land/0.56.0/Contributing-and-Debugging/Translations/)
- [Crashes-and-Bugs](https://wiki.hypr.land/0.56.0/Crashes-and-Bugs/)
- [FAQ](https://wiki.hypr.land/0.56.0/FAQ/)
- [Getting-Started](https://wiki.hypr.land/0.56.0/Getting-Started/)
- [Getting-Started/Installation](https://wiki.hypr.land/0.56.0/Getting-Started/Installation/)
- [Getting-Started/Master-Tutorial](https://wiki.hypr.land/0.56.0/Getting-Started/Master-Tutorial/)
- [Getting-Started/Preconfigured-setups](https://wiki.hypr.land/0.56.0/Getting-Started/Preconfigured-setups/)
- [Hypr-Ecosystem](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/)
- [Hypr-Ecosystem/aquamarine](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/aquamarine/)
- [Hypr-Ecosystem/hyprcursor](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprcursor/)
- [Hypr-Ecosystem/hyprgraphics](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprgraphics/)
- [Hypr-Ecosystem/hypridle](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hypridle/)
- [Hypr-Ecosystem/hyprland-guiutils](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprland-guiutils/)
- [Hypr-Ecosystem/hyprland-qt-support](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprland-qt-support/)
- [Hypr-Ecosystem/hyprlang](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprlang/)
- [Hypr-Ecosystem/hyprlauncher](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprlauncher/)
- [Hypr-Ecosystem/hyprlock](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprlock/)
- [Hypr-Ecosystem/hyprpaper](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprpaper/)
- [Hypr-Ecosystem/hyprpicker](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprpicker/)
- [Hypr-Ecosystem/hyprpolkitagent](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprpolkitagent/)
- [Hypr-Ecosystem/hyprpwcenter](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprpwcenter/)
- [Hypr-Ecosystem/hyprqt6engine](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprqt6engine/)
- [Hypr-Ecosystem/hyprshutdown](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprshutdown/)
- [Hypr-Ecosystem/hyprsunset](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprsunset/)
- [Hypr-Ecosystem/hyprsysteminfo](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprsysteminfo/)
- [Hypr-Ecosystem/hyprtoolkit](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprtoolkit/)
- [Hypr-Ecosystem/hyprtoolkit/development](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprtoolkit/development/)
- [Hypr-Ecosystem/hyprutils](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprutils/)
- [Hypr-Ecosystem/hyprwayland-scanner](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprwayland-scanner/)
- [Hypr-Ecosystem/xdg-desktop-portal-hyprland](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/xdg-desktop-portal-hyprland/)
- [IPC](https://wiki.hypr.land/0.56.0/IPC/)
- [Nix](https://wiki.hypr.land/0.56.0/Nix/)
- [Nix/Cachix](https://wiki.hypr.land/0.56.0/Nix/Cachix/)
- [Nix/Contributing-and-Debugging](https://wiki.hypr.land/0.56.0/Nix/Contributing-and-Debugging/)
- [Nix/Hyprland-on-Home-Manager](https://wiki.hypr.land/0.56.0/Nix/Hyprland-on-Home-Manager/)
- [Nix/Hyprland-on-NixOS](https://wiki.hypr.land/0.56.0/Nix/Hyprland-on-NixOS/)
- [Nix/Hyprland-on-other-distros](https://wiki.hypr.land/0.56.0/Nix/Hyprland-on-other-distros/)
- [Nix/Options-Overrides](https://wiki.hypr.land/0.56.0/Nix/Options-Overrides/)
- [Nix/Plugins](https://wiki.hypr.land/0.56.0/Nix/Plugins/)
- [Nvidia](https://wiki.hypr.land/0.56.0/Nvidia/)
- [Plugins](https://wiki.hypr.land/0.56.0/Plugins/)
- [Plugins/Development](https://wiki.hypr.land/0.56.0/Plugins/Development/)
- [Plugins/Development/Advanced](https://wiki.hypr.land/0.56.0/Plugins/Development/Advanced/)
- [Plugins/Development/Getting-Started](https://wiki.hypr.land/0.56.0/Plugins/Development/Getting-Started/)
- [Plugins/Development/Plugin-Guidelines](https://wiki.hypr.land/0.56.0/Plugins/Development/Plugin-Guidelines/)
- [Plugins/Using-Plugins](https://wiki.hypr.land/0.56.0/Plugins/Using-Plugins/)
- [Useful-Utilities](https://wiki.hypr.land/0.56.0/Useful-Utilities/)
- [Useful-Utilities/App-Clients](https://wiki.hypr.land/0.56.0/Useful-Utilities/App-Clients/)
- [Useful-Utilities/App-Launchers](https://wiki.hypr.land/0.56.0/Useful-Utilities/App-Launchers/)
- [Useful-Utilities/Clipboard-Managers](https://wiki.hypr.land/0.56.0/Useful-Utilities/Clipboard-Managers/)
- [Useful-Utilities/Color-Pickers](https://wiki.hypr.land/0.56.0/Useful-Utilities/Color-Pickers/)
- [Useful-Utilities/File-Managers](https://wiki.hypr.land/0.56.0/Useful-Utilities/File-Managers/)
- [Useful-Utilities/Hypr-Ecosystem](https://wiki.hypr.land/0.56.0/Useful-Utilities/Hypr-Ecosystem/)
- [Useful-Utilities/Must-have](https://wiki.hypr.land/0.56.0/Useful-Utilities/Must-have/)
- [Useful-Utilities/Other](https://wiki.hypr.land/0.56.0/Useful-Utilities/Other/)
- [Useful-Utilities/Phone-connect](https://wiki.hypr.land/0.56.0/Useful-Utilities/Phone-connect/)
- [Useful-Utilities/Screen-Sharing](https://wiki.hypr.land/0.56.0/Useful-Utilities/Screen-Sharing/)
- [Useful-Utilities/Screenshots-and-Recording](https://wiki.hypr.land/0.56.0/Useful-Utilities/Screenshots-and-Recording/)
- [Useful-Utilities/Status-Bars](https://wiki.hypr.land/0.56.0/Useful-Utilities/Status-Bars/)
- [Useful-Utilities/Systemd-start](https://wiki.hypr.land/0.56.0/Useful-Utilities/Systemd-start/)
- [Useful-Utilities/Wallpapers](https://wiki.hypr.land/0.56.0/Useful-Utilities/Wallpapers/)
- [version-selector](https://wiki.hypr.land/0.56.0/version-selector/)

## Verification limits

No GUI was built or installed. No live setting was changed. No memory/GPU benchmark or sandbox penetration test was performed. This is the complete discovered baseline inventory for planning, with dynamic and version-dependent extension contracts explicitly listed in the plan.

## Current git source comparison

Current source: [6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7](https://github.com/hyprwm/Hyprland/tree/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7). This is a separate latest-git schema candidate, not the installed release.

402 scalar settings; 50 added, 1 removed and 17 common entries with changed type/default/metadata relative to 0.56.2. Metadata includes bounds, maps and refresh flags; this is not a count of semantic breakages.

**Added:** `binds:drag_center_window`, `cursor:warp_on_monitor_change`, `debug:invalidate_buffers`, `decoration:blur:acrylic:aberration`, `decoration:blur:acrylic:bulb`, `decoration:blur:acrylic:clarity`, `decoration:blur:acrylic:refraction`, `decoration:blur:acrylic:tint`, `decoration:blur:aurora:color1`, `decoration:blur:aurora:color2`, `decoration:blur:aurora:intensity`, `decoration:blur:aurora:speed`, `decoration:blur:drops:speed`, `decoration:blur:fluid_jar:color`, `decoration:blur:fluid_jar:distortion`, `decoration:blur:fluid_jar:fill_amount`, `decoration:blur:fluid_jar:mass`, `decoration:blur:fluid_jar:precision`, `decoration:blur:fluid_jar:speed`, `decoration:blur:fluid_jar:turbulence`, `decoration:blur:glass:refraction`, `decoration:blur:glass:roughness`, `decoration:blur:glass:size`, `decoration:blur:haze:intensity`, `decoration:blur:haze:iridescence`, `decoration:blur:heat_shimmer:speed`, `decoration:blur:ripple:duration`, `decoration:blur:ripple:radius`, `decoration:blur:ripple:strength`, `decoration:blur:ripple:width`, `decoration:blur:variant`, `decoration:blur:water:damping`, `decoration:blur:water:duration`, `decoration:blur:water:radius`, `decoration:blur:water:speed`, `decoration:blur:water:strength`, `decoration:wobble:damping`, `decoration:wobble:enabled`, `decoration:wobble:intensity`, `decoration:wobble:mass`, `decoration:wobble:mesh`, `decoration:wobble:stiffness`, `decoration:wobble:value_epsilon`, `decoration:wobble:velocity_epsilon`, `input:tablet:enabled`, `misc:bell_sound`, `misc:float_force_onscreen`, `misc:new_float_force_onscreen`, `render:async_commit`, `render:not_shown_fifo_lock`.

**Removed:** `debug:fifo_pending_workaround`.

**Changed type/default/metadata:** `binds:workspace_back_and_forth`, `debug:invalidate_fp16`, `decoration:rounding`, `decoration:rounding_power`, `decoration:shadow:scale`, `dwindle:special_scale_factor`, `group:drag_into_group`, `group:groupbar:gradient_rounding`, `group:groupbar:gradient_rounding_power`, `group:groupbar:indicator_height`, `group:groupbar:rounding`, `group:groupbar:rounding_power`, `input:scroll_factor`, `input:touchpad:scroll_factor`, `master:special_scale_factor`, `misc:exit_window_retains_fullscreen`, `quirks:skip_non_kms_dmabuf_formats`.

| Current source key | Type | Default expression | Planned control |
|---|---|---|---|
| [animations:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L346) | Bool | `true` | switch |
| [animations:workspace_wraparound](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L347) | Bool | `false` | switch |
| [binds:allow_pin_fullscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L628) | Bool | `false` | switch |
| [binds:allow_workspace_cycles](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L618) | Bool | `false` | switch |
| [binds:disable_keybind_grabbing](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L625) | Bool | `false` | switch |
| [binds:drag_center_window](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L631) | Bool | `true` | switch |
| [binds:drag_threshold](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L629) | Int | `0` | integer control; enumeration picker when a value map exists |
| [binds:focus_preferred_method](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L621) | Int | `0` | integer control; enumeration picker when a value map exists |
| [binds:hide_special_on_workspace_change](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L617) | Bool | `false` | switch |
| [binds:ignore_group_lock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L622) | Bool | `false` | switch |
| [binds:movefocus_cycles_fullscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L623) | Bool | `false` | switch |
| [binds:movefocus_cycles_groupfirst](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L624) | Bool | `false` | switch |
| [binds:pass_mouse_when_bound](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L613) | Bool | `false` | switch |
| [binds:scroll_event_delay](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L614) | Int | `300` | integer control; enumeration picker when a value map exists |
| [binds:window_direction_monitor_fallback](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L626) | Bool | `true` | switch |
| [binds:workspace_back_and_forth](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L615) | Int | `0` | integer control; enumeration picker when a value map exists |
| [binds:workspace_center_on](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L619) | Int | `1` | integer control; enumeration picker when a value map exists |
| [cursor:default_monitor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L698) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [cursor:enable_hyprcursor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L703) | Bool | `true` | switch |
| [cursor:hide_on_key_press](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L704) | Bool | `false` | switch |
| [cursor:hide_on_tablet](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L706) | Bool | `false` | switch |
| [cursor:hide_on_touch](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L705) | Bool | `true` | switch |
| [cursor:hotspot_padding](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L688) | Int | `0` | integer control; enumeration picker when a value map exists |
| [cursor:inactive_timeout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L689) | Float | `0` | decimal control |
| [cursor:invisible](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L683) | Bool | `false` | switch |
| [cursor:min_refresh_rate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L687) | Int | `24` | integer control; enumeration picker when a value map exists |
| [cursor:no_break_fs_vrr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L685) | Int | `2` | integer control; enumeration picker when a value map exists |
| [cursor:no_hardware_cursors](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L684) | Int | `2` | integer control; enumeration picker when a value map exists |
| [cursor:no_warps](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L690) | Bool | `false` | switch |
| [cursor:persistent_warps](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L691) | Bool | `false` | switch |
| [cursor:sync_gsettings_theme](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L708) | Bool | `true` | switch |
| [cursor:use_cpu_buffer](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L707) | Int | `2` | integer control; enumeration picker when a value map exists |
| [cursor:warp_back_after_non_mouse_input](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L709) | Bool | `false` | switch |
| [cursor:warp_on_change_workspace](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L692) | Int | `0` | integer control; enumeration picker when a value map exists |
| [cursor:warp_on_monitor_change](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L694) | Int | `-1` | integer control; enumeration picker when a value map exists |
| [cursor:warp_on_toggle_special](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L696) | Int | `0` | integer control; enumeration picker when a value map exists |
| [cursor:zoom_detached_camera](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L702) | Bool | `true` | switch |
| [cursor:zoom_disable_aa](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L701) | Bool | `false` | switch |
| [cursor:zoom_factor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L699) | Float | `1` | decimal control |
| [cursor:zoom_rigid](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L700) | Bool | `false` | switch |
| [debug:colored_stdout_logs](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L735) | Bool | `true` | switch |
| [debug:damage_blink](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L724) | Bool | `false` | switch |
| [debug:damage_tracking](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L728) | Int | `2` | integer control; enumeration picker when a value map exists |
| [debug:disable_logs](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L726) | Bool | `true` | switch |
| [debug:disable_scale_checks](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L732) | Bool | `false` | switch |
| [debug:disable_time](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L727) | Bool | `true` | switch |
| [debug:ds_handle_same_buffer](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L739) | Bool | `true` | switch |
| [debug:ds_handle_same_buffer_fifo](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L740) | Bool | `true` | switch |
| [debug:enable_stdout_logs](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L729) | Bool | `false` | switch |
| [debug:error_limit](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L733) | Int | `5` | integer control; enumeration picker when a value map exists |
| [debug:error_position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L734) | Int | `0` | integer control; enumeration picker when a value map exists |
| [debug:full_cm_proto](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L738) | Bool | `false` | switch |
| [debug:gl_debugging](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L725) | Bool | `false` | switch |
| [debug:invalidate_buffers](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L743) | Int | `1` | integer control; enumeration picker when a value map exists |
| [debug:invalidate_fp16](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L744) | Int | `1` | integer control; enumeration picker when a value map exists |
| [debug:log_damage](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L736) | Bool | `false` | switch |
| [debug:manual_crash](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L730) | Int | `0` | integer control; enumeration picker when a value map exists |
| [debug:overlay](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L723) | Bool | `false` | switch |
| [debug:pass](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L737) | Bool | `false` | switch |
| [debug:render_solitary_wo_damage](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L741) | Bool | `false` | switch |
| [debug:suppress_errors](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L731) | Bool | `false` | switch |
| [debug:vfr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L742) | Bool | `true` | switch |
| [decoration:active_opacity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L204) | Float | `1` | decimal control |
| [decoration:blur:acrylic:aberration](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L286) | Float | `0.025F` | decimal control |
| [decoration:blur:acrylic:bulb](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L283) | Float | `48.F` | decimal control |
| [decoration:blur:acrylic:clarity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L284) | Float | `0.82F` | decimal control |
| [decoration:blur:acrylic:refraction](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L282) | Float | `24.F` | decimal control |
| [decoration:blur:acrylic:tint](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L288) | Color | `0x14EEF5FF` | color picker |
| [decoration:blur:aurora:color1](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L299) | Color | `0x29F0A0FF` | color picker |
| [decoration:blur:aurora:color2](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L300) | Color | `0x7A4DFFFF` | color picker |
| [decoration:blur:aurora:intensity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L298) | Float | `0.35F` | decimal control |
| [decoration:blur:aurora:speed](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L296) | Float | `1.F` | decimal control |
| [decoration:blur:brightness](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L265) | Float | `1` | decimal control |
| [decoration:blur:contrast](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L264) | Float | `0.8916` | decimal control |
| [decoration:blur:drops:speed](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L290) | Float | `3.F` | decimal control |
| [decoration:blur:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L238) | Bool | `true` | switch |
| [decoration:blur:fluid_jar:color](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L318) | Color | `0xCC3399FF` | color picker |
| [decoration:blur:fluid_jar:distortion](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L325) | Float | `8.F` | decimal control |
| [decoration:blur:fluid_jar:fill_amount](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L320) | Float | `0.5F` | decimal control |
| [decoration:blur:fluid_jar:mass](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L321) | Float | `1.4F` | decimal control |
| [decoration:blur:fluid_jar:precision](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L322) | Float | `2.F` | decimal control |
| [decoration:blur:fluid_jar:speed](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L319) | Float | `3.7F` | decimal control |
| [decoration:blur:fluid_jar:turbulence](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L324) | Float | `1.2F` | decimal control |
| [decoration:blur:glass:refraction](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L277) | Float | `20.F` | decimal control |
| [decoration:blur:glass:roughness](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L280) | Float | `1.F` | decimal control |
| [decoration:blur:glass:size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L279) | Float | `40.F` | decimal control |
| [decoration:blur:haze:intensity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L302) | Float | `0.35F` | decimal control |
| [decoration:blur:haze:iridescence](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L303) | Float | `0.7F` | decimal control |
| [decoration:blur:heat_shimmer:speed](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L293) | Float | `1.F` | decimal control |
| [decoration:blur:ignore_opacity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L260) | Bool | `true` | switch |
| [decoration:blur:input_methods](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L272) | Bool | `false` | switch |
| [decoration:blur:input_methods_ignorealpha](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L273) | Float | `0.2` | decimal control |
| [decoration:blur:new_optimizations](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L261) | Bool | `true` | switch |
| [decoration:blur:noise](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L263) | Float | `0.0117` | decimal control |
| [decoration:blur:passes](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L259) | Int | `1` | integer control; enumeration picker when a value map exists |
| [decoration:blur:popups](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L269) | Bool | `false` | switch |
| [decoration:blur:popups_ignorealpha](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L270) | Float | `0.2` | decimal control |
| [decoration:blur:ripple:duration](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L309) | Float | `0.45F` | decimal control |
| [decoration:blur:ripple:radius](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L307) | Float | `400.F` | decimal control |
| [decoration:blur:ripple:strength](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L305) | Float | `30.F` | decimal control |
| [decoration:blur:ripple:width](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L308) | Float | `32.F` | decimal control |
| [decoration:blur:size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L258) | Int | `8` | integer control; enumeration picker when a value map exists |
| [decoration:blur:special](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L268) | Bool | `false` | switch |
| [decoration:blur:variant](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L239) | Int | `0` | integer control; enumeration picker when a value map exists |
| [decoration:blur:vibrancy](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L266) | Float | `0.1696` | decimal control |
| [decoration:blur:vibrancy_darkness](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L267) | Float | `0` | decimal control |
| [decoration:blur:water:damping](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L315) | Float | `0.95F` | decimal control |
| [decoration:blur:water:duration](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L316) | Float | `12.F` | decimal control |
| [decoration:blur:water:radius](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L313) | Float | `20.F` | decimal control |
| [decoration:blur:water:speed](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L314) | Float | `0.76F` | decimal control |
| [decoration:blur:water:strength](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L311) | Float | `32.F` | decimal control |
| [decoration:blur:xray](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L262) | Bool | `false` | switch |
| [decoration:border_part_of_window](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L232) | Bool | `true` | switch |
| [decoration:dim_around](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L230) | Float | `0.4` | decimal control |
| [decoration:dim_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L226) | Bool | `false` | switch |
| [decoration:dim_modal](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L225) | Bool | `true` | switch |
| [decoration:dim_special](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L228) | Float | `0.2` | decimal control |
| [decoration:dim_strength](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L227) | Float | `0.5` | decimal control |
| [decoration:fullscreen_opacity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L206) | Float | `1` | decimal control |
| [decoration:glow:color](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L222) | Gradient | `CHyprColor{0xee33ccff}` | gradient stop/angle editor |
| [decoration:glow:color_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L223) | Gradient | `-1` | gradient stop/angle editor |
| [decoration:glow:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L218) | Bool | `false` | switch |
| [decoration:glow:range](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L219) | Int | `10` | integer control; enumeration picker when a value map exists |
| [decoration:glow:render_power](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L220) | Int | `3` | integer control; enumeration picker when a value map exists |
| [decoration:inactive_opacity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L205) | Float | `1` | decimal control |
| [decoration:motion_blur:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L331) | Bool | `false` | switch |
| [decoration:motion_blur:samples](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L332) | Int | `7` | integer control; enumeration picker when a value map exists |
| [decoration:rounding](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L200) | Int | `0` | integer control; enumeration picker when a value map exists |
| [decoration:rounding_power](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L202) | Float | `2` | decimal control |
| [decoration:screen_shader](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L231) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [decoration:shadow:color](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L212) | Gradient | `CHyprColor{0xee1a1a1a}` | gradient stop/angle editor |
| [decoration:shadow:color_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L213) | Gradient | `-1` | gradient stop/angle editor |
| [decoration:shadow:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L207) | Bool | `true` | switch |
| [decoration:shadow:offset](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L215) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [decoration:shadow:range](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L208) | Int | `4` | integer control; enumeration picker when a value map exists |
| [decoration:shadow:render_power](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L209) | Int | `3` | integer control; enumeration picker when a value map exists |
| [decoration:shadow:scale](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L217) | Float | `1` | decimal control |
| [decoration:shadow:sharp](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L211) | Bool | `false` | switch |
| [decoration:wobble:damping](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L336) | Float | `12` | decimal control |
| [decoration:wobble:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L333) | Bool | `false` | switch |
| [decoration:wobble:intensity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L338) | Float | `0.2` | decimal control |
| [decoration:wobble:mass](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L337) | Float | `1` | decimal control |
| [decoration:wobble:mesh](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L334) | Int | `12` | integer control; enumeration picker when a value map exists |
| [decoration:wobble:stiffness](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L335) | Float | `200` | decimal control |
| [decoration:wobble:value_epsilon](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L339) | Float | `0.25` | decimal control |
| [decoration:wobble:velocity_epsilon](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L340) | Float | `2` | decimal control |
| [dwindle:default_split_ratio](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L772) | Float | `1` | decimal control |
| [dwindle:force_split](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L763) | Int | `0` | integer control; enumeration picker when a value map exists |
| [dwindle:permanent_direction_override](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L767) | Bool | `false` | switch |
| [dwindle:precise_mouse_move](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L774) | Bool | `false` | switch |
| [dwindle:preserve_split](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L764) | Bool | `false` | switch |
| [dwindle:smart_resizing](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L766) | Bool | `true` | switch |
| [dwindle:smart_split](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L765) | Bool | `false` | switch |
| [dwindle:special_scale_factor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L768) | Float | `1` | decimal control |
| [dwindle:split_bias](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L773) | Int | `0` | integer control; enumeration picker when a value map exists |
| [dwindle:split_width_multiplier](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L770) | Float | `1` | decimal control |
| [dwindle:use_active_for_splits](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L771) | Bool | `true` | switch |
| [ecosystem:enforce_permissions](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L717) | Bool | `false` | switch |
| [ecosystem:no_donation_nag](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L716) | Bool | `false` | switch |
| [ecosystem:no_update_news](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L715) | Bool | `false` | switch |
| [experimental:wp_cm_1_2](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L820) | Bool | `true` | switch |
| [general:allow_tearing](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L185) | Bool | `false` | switch |
| [general:border_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L168) | Int | `1` | integer control; enumeration picker when a value map exists |
| [general:col.active_border](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L174) | Gradient | `CHyprColor{0xffffffff}` | gradient stop/angle editor |
| [general:col.inactive_border](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L173) | Gradient | `CHyprColor{0xff444444}` | gradient stop/angle editor |
| [general:col.nogroup_border](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L175) | Gradient | `CHyprColor{0xffffaaff}` | gradient stop/angle editor |
| [general:col.nogroup_border_active](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L177) | Gradient | `CHyprColor{0xffff00ff}` | gradient stop/angle editor |
| [general:extend_border_grab_area](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L182) | Int | `15` | integer control; enumeration picker when a value map exists |
| [general:float_gaps](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L171) | CssGap | `0` | four-side spacing control |
| [general:gaps_in](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L169) | CssGap | `5` | four-side spacing control |
| [general:gaps_out](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L170) | CssGap | `20` | four-side spacing control |
| [general:gaps_workspaces](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L172) | Int | `0` | integer control; enumeration picker when a value map exists |
| [general:hover_icon_on_border](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L184) | Bool | `true` | switch |
| [general:layout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L179) | String | `"dwindle"` | validated text/picker; dedicated builder for specialized syntax |
| [general:locale](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L194) | String | `""` | validated text/picker; dedicated builder for specialized syntax |
| [general:modal_parent_blocking](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L193) | Bool | `true` | switch |
| [general:no_focus_fallback](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L180) | Bool | `false` | switch |
| [general:resize_corner](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L186) | Int | `0` | integer control; enumeration picker when a value map exists |
| [general:resize_on_border](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L181) | Bool | `false` | switch |
| [general:snap:border_overlap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L191) | Bool | `false` | switch |
| [general:snap:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L188) | Bool | `false` | switch |
| [general:snap:monitor_gap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L190) | Int | `10` | integer control; enumeration picker when a value map exists |
| [general:snap:respect_gaps](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L192) | Bool | `false` | switch |
| [general:snap:window_gap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L189) | Int | `10` | integer control; enumeration picker when a value map exists |
| [gestures:close_max_timeout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L495) | Int | `1000` | integer control; enumeration picker when a value map exists |
| [gestures:scrolling:move_snap_cursor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L497) | Bool | `true` | switch |
| [gestures:scrolling:move_snap_to_grid](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L496) | Bool | `true` | switch |
| [gestures:workspace_swipe_cancel_ratio](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L489) | Float | `0.5` | decimal control |
| [gestures:workspace_swipe_create_new](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L490) | Bool | `true` | switch |
| [gestures:workspace_swipe_direction_lock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L491) | Bool | `true` | switch |
| [gestures:workspace_swipe_direction_lock_threshold](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L492) | Int | `10` | integer control; enumeration picker when a value map exists |
| [gestures:workspace_swipe_distance](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L484) | Int | `300` | integer control; enumeration picker when a value map exists |
| [gestures:workspace_swipe_forever](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L493) | Bool | `false` | switch |
| [gestures:workspace_swipe_invert](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L486) | Bool | `true` | switch |
| [gestures:workspace_swipe_min_speed_to_force](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L488) | Int | `30` | integer control; enumeration picker when a value map exists |
| [gestures:workspace_swipe_touch](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L485) | Bool | `false` | switch |
| [gestures:workspace_swipe_touch_invert](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L487) | Bool | `false` | switch |
| [gestures:workspace_swipe_use_r](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L494) | Bool | `false` | switch |
| [group:auto_group](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L510) | Bool | `true` | switch |
| [group:col.border_active](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L506) | Gradient | `CHyprColor{0x66ffff00}` | gradient stop/angle editor |
| [group:col.border_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L507) | Gradient | `CHyprColor{0x66777700}` | gradient stop/angle editor |
| [group:col.border_locked_active](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L509) | Gradient | `CHyprColor{0x66775500}` | gradient stop/angle editor |
| [group:col.border_locked_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L508) | Gradient | `CHyprColor{0x66ff5500}` | gradient stop/angle editor |
| [group:drag_into_group](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L511) | Int | `1` | integer control; enumeration picker when a value map exists |
| [group:focus_removed_window](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L503) | Bool | `true` | switch |
| [group:group_on_movetoworkspace](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L514) | Bool | `false` | switch |
| [group:groupbar:blur](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L555) | Bool | `false` | switch |
| [group:groupbar:col.active](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L546) | Gradient | `0x66ffff00` | gradient stop/angle editor |
| [group:groupbar:col.inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L547) | Gradient | `0x66777700` | gradient stop/angle editor |
| [group:groupbar:col.locked_active](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L548) | Gradient | `0x66ff5500` | gradient stop/angle editor |
| [group:groupbar:col.locked_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L549) | Gradient | `0x66775500` | gradient stop/angle editor |
| [group:groupbar:disable_when_only](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L521) | Bool | `false` | switch |
| [group:groupbar:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L520) | Bool | `true` | switch |
| [group:groupbar:font_family](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L523) | String | `"[[EMPTY]]"` | validated text/picker; dedicated builder for specialized syntax |
| [group:groupbar:font_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L526) | Int | `8` | integer control; enumeration picker when a value map exists |
| [group:groupbar:font_weight_active](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L524) | FontWeight | `400 /* FontWeight constructor default */` | weight selector |
| [group:groupbar:font_weight_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L525) | FontWeight | `400 /* FontWeight constructor default */` | weight selector |
| [group:groupbar:gaps_in](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L551) | Int | `2` | integer control; enumeration picker when a value map exists |
| [group:groupbar:gaps_out](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L550) | Int | `2` | integer control; enumeration picker when a value map exists |
| [group:groupbar:gradient_round_only_edges](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L541) | Bool | `true` | switch |
| [group:groupbar:gradient_rounding](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L538) | Int | `2` | integer control; enumeration picker when a value map exists |
| [group:groupbar:gradient_rounding_power](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L539) | Float | `2` | decimal control |
| [group:groupbar:gradients](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L527) | Bool | `false` | switch |
| [group:groupbar:height](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L528) | Int | `14` | integer control; enumeration picker when a value map exists |
| [group:groupbar:indicator_gap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L529) | Int | `0` | integer control; enumeration picker when a value map exists |
| [group:groupbar:indicator_height](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L530) | Int | `3` | integer control; enumeration picker when a value map exists |
| [group:groupbar:keep_upper_gap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L552) | Bool | `true` | switch |
| [group:groupbar:middle_click_close](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L535) | Bool | `true` | switch |
| [group:groupbar:priority](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L532) | Int | `3` | integer control; enumeration picker when a value map exists |
| [group:groupbar:render_titles](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L533) | Bool | `true` | switch |
| [group:groupbar:round_only_edges](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L540) | Bool | `true` | switch |
| [group:groupbar:rounding](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L536) | Int | `1` | integer control; enumeration picker when a value map exists |
| [group:groupbar:rounding_power](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L537) | Float | `2` | decimal control |
| [group:groupbar:scrolling](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L534) | Bool | `true` | switch |
| [group:groupbar:stacked](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L531) | Bool | `false` | switch |
| [group:groupbar:text_color](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L542) | Color | `0xffffffff` | color picker |
| [group:groupbar:text_color_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L543) | Color | `-1` | color picker |
| [group:groupbar:text_color_locked_active](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L544) | Color | `-1` | color picker |
| [group:groupbar:text_color_locked_inactive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L545) | Color | `-1` | color picker |
| [group:groupbar:text_offset](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L553) | Int | `0` | integer control; enumeration picker when a value map exists |
| [group:groupbar:text_padding](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L554) | Int | `0` | integer control; enumeration picker when a value map exists |
| [group:insert_after_current](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L502) | Bool | `true` | switch |
| [group:merge_floated_into_tiled_on_groupbar](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L513) | Bool | `false` | switch |
| [group:merge_groups_on_drag](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L504) | Bool | `true` | switch |
| [group:merge_groups_on_groupbar](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L505) | Bool | `true` | switch |
| [input-capture:capture_modifiers](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L825) | Bool | `false` | switch |
| [input-capture:enforce_barriers](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L826) | Bool | `true` | switch |
| [input:accel_profile](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L365) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:emulate_discrete_scroll](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L393) | Int | `1` | integer control; enumeration picker when a value map exists |
| [input:float_switch_override_focus](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L386) | Int | `1` | integer control; enumeration picker when a value map exists |
| [input:focus_on_close](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L383) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:follow_mouse](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L380) | Int | `1` | integer control; enumeration picker when a value map exists |
| [input:follow_mouse_shrink](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L395) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:follow_mouse_threshold](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L382) | Float | `0` | decimal control |
| [input:force_no_accel](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L367) | Bool | `false` | switch |
| [input:kb_file](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L358) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_layout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L354) | String | `"us"` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_model](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L353) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_options](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L356) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_rules](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L357) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:kb_variant](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L355) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:left_handed](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L370) | Bool | `false` | switch |
| [input:mouse_refocus](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L385) | Bool | `true` | switch |
| [input:natural_scroll](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L379) | Bool | `false` | switch |
| [input:numlock_by_default](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L359) | Bool | `false` | switch |
| [input:off_window_axis_events](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L391) | Int | `1` | integer control; enumeration picker when a value map exists |
| [input:repeat_delay](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L362) | Int | `600` | integer control; enumeration picker when a value map exists |
| [input:repeat_rate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L361) | Int | `25` | integer control; enumeration picker when a value map exists |
| [input:resolve_binds_by_sym](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L360) | Bool | `false` | switch |
| [input:rotation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L368) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:scroll_button](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L375) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:scroll_button_lock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L376) | Bool | `false` | switch |
| [input:scroll_factor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L378) | Float | `1` | decimal control |
| [input:scroll_method](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L373) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:scroll_points](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L371) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:sensitivity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L363) | Float | `0` | decimal control |
| [input:special_fallthrough](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L390) | Bool | `false` | switch |
| [input:tablet:absolute_region_position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L450) | Bool | `false` | switch |
| [input:tablet:active_area_position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L458) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [input:tablet:active_area_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L456) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [input:tablet:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L445) | Bool | `true` | switch |
| [input:tablet:left_handed](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L455) | Bool | `false` | switch |
| [input:tablet:output](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L447) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:tablet:region_position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L448) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [input:tablet:region_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L452) | Vec2 | `Config::VEC2{}` | paired coordinate controls |
| [input:tablet:relative_input](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L454) | Bool | `false` | switch |
| [input:tablet:transform](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L446) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:tablettool:eraser_button_mode](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L465) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:tablettool:eraser_button_override](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L469) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:tablettool:pressure_range_max](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L476) | Float | `-1.0` | decimal control |
| [input:tablettool:pressure_range_min](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L473) | Float | `-1.0` | decimal control |
| [input:touchdevice:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L430) | Bool | `true` | switch |
| [input:touchdevice:output](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L429) | String | `"[[Auto]]"` | validated text/picker; dedicated builder for specialized syntax |
| [input:touchdevice:transform](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L428) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:touchpad:clickfinger_behavior](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L412) | Bool | `false` | switch |
| [input:touchpad:disable_while_typing](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L404) | Bool | `true` | switch |
| [input:touchpad:drag_3fg](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L421) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:touchpad:drag_lock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L416) | Int | `0` | integer control; enumeration picker when a value map exists |
| [input:touchpad:flip_x](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L419) | Bool | `false` | switch |
| [input:touchpad:flip_y](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L420) | Bool | `false` | switch |
| [input:touchpad:middle_button_emulation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L408) | Bool | `false` | switch |
| [input:touchpad:natural_scroll](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L405) | Bool | `false` | switch |
| [input:touchpad:scroll_factor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L406) | Float | `1` | decimal control |
| [input:touchpad:tap-and-drag](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L418) | Bool | `true` | switch |
| [input:touchpad:tap-to-click](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L414) | Bool | `true` | switch |
| [input:touchpad:tap_button_map](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L410) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [input:virtualkeyboard:release_pressed_on_close](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L438) | Bool | `false` | switch |
| [input:virtualkeyboard:share_states](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L436) | Int | `2` | integer control; enumeration picker when a value map exists |
| [layout:single_window_aspect_ratio](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L754) | Vec2 | `Config::VEC2{0, 0}` | paired coordinate controls |
| [layout:single_window_aspect_ratio_tolerance](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L756) | Float | `0.1F` | decimal control |
| [master:allow_small_split](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L780) | Bool | `false` | switch |
| [master:always_keep_position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L795) | Bool | `false` | switch |
| [master:center_ignores_reserved](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L792) | Bool | `false` | switch |
| [master:center_master_fallback](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L790) | String | `"left"` | validated text/picker; dedicated builder for specialized syntax |
| [master:drop_at_cursor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L794) | Bool | `true` | switch |
| [master:focus_master_on_close](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L797) | Bool | `false` | switch |
| [master:mfact](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L783) | Float | `0.55` | decimal control |
| [master:new_on_active](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L786) | String | `"none"` | validated text/picker; dedicated builder for specialized syntax |
| [master:new_on_top](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L785) | Bool | `false` | switch |
| [master:new_status](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L784) | String | `"slave"` | validated text/picker; dedicated builder for specialized syntax |
| [master:orientation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L787) | String | `"left"` | validated text/picker; dedicated builder for specialized syntax |
| [master:slave_count_for_center_master](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L788) | Int | `2` | integer control; enumeration picker when a value map exists |
| [master:smart_resizing](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L793) | Bool | `true` | switch |
| [master:special_scale_factor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L781) | Float | `1` | decimal control |
| [misc:allow_session_lock_restore](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L582) | Bool | `false` | switch |
| [misc:always_follow_on_dnd](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L572) | Bool | `true` | switch |
| [misc:animate_manual_resizes](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L574) | Bool | `false` | switch |
| [misc:animate_mouse_windowdragging](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L575) | Bool | `false` | switch |
| [misc:anr_missed_pings](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L601) | Int | `5` | integer control; enumeration picker when a value map exists |
| [misc:background_color](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L585) | Color | `0xff111111` | color picker |
| [misc:bell_sound](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L605) | String | `"default"` | validated text/picker; dedicated builder for specialized syntax |
| [misc:close_special_on_empty](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L586) | Bool | `true` | switch |
| [misc:col.splash](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L563) | Color | `0x55ffffff` | color picker |
| [misc:disable_autoreload](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L576) | Bool | `false` | switch |
| [misc:disable_hyprland_guiutils_check](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L597) | Bool | `false` | switch |
| [misc:disable_hyprland_logo](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L561) | Bool | `false` | switch |
| [misc:disable_scale_notification](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L603) | Bool | `false` | switch |
| [misc:disable_splash_rendering](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L562) | Bool | `false` | switch |
| [misc:disable_watchdog_warning](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L598) | Bool | `false` | switch |
| [misc:disable_xdg_env_checks](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L596) | Bool | `false` | switch |
| [misc:enable_anr_dialog](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L600) | Bool | `true` | switch |
| [misc:enable_swallow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L577) | Bool | `false` | switch |
| [misc:exit_window_retains_fullscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L589) | Int | `0` | integer control; enumeration picker when a value map exists |
| [misc:float_force_onscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L607) | Int | `0` | integer control; enumeration picker when a value map exists |
| [misc:focus_on_activate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L580) | Bool | `false` | switch |
| [misc:font_family](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L564) | String | `"Sans"` | validated text/picker; dedicated builder for specialized syntax |
| [misc:force_default_wallpaper](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L566) | Int | `-1` | integer control; enumeration picker when a value map exists |
| [misc:initial_workspace_token_timeout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L592) | Int | `10` | integer control; enumeration picker when a value map exists |
| [misc:initial_workspace_tracking](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L591) | Int | `1` | integer control; enumeration picker when a value map exists |
| [misc:key_press_enables_dpms](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L570) | Bool | `false` | switch |
| [misc:layers_hog_keyboard_focus](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L573) | Bool | `true` | switch |
| [misc:lockdead_screen_delay](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L599) | Int | `1000` | integer control; enumeration picker when a value map exists |
| [misc:middle_click_paste](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L594) | Bool | `true` | switch |
| [misc:mouse_move_enables_dpms](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L569) | Bool | `false` | switch |
| [misc:mouse_move_focuses_monitor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L581) | Bool | `true` | switch |
| [misc:name_vk_after_proc](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L571) | Bool | `true` | switch |
| [misc:new_float_force_onscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L606) | Int | `2` | integer control; enumeration picker when a value map exists |
| [misc:on_focus_under_fullscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L587) | Int | `2` | integer control; enumeration picker when a value map exists |
| [misc:render_unfocused_fps](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L595) | Int | `15` | integer control; enumeration picker when a value map exists |
| [misc:screencopy_force_8b](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L602) | Bool | `true` | switch |
| [misc:session_lock_blur](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L584) | Bool | `false` | switch |
| [misc:session_lock_xray](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L583) | Bool | `false` | switch |
| [misc:size_limits_tiled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L604) | Bool | `false` | switch |
| [misc:splash_font_family](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L565) | String | `"[[EMPTY]]"` | validated text/picker; dedicated builder for specialized syntax |
| [misc:swallow_exception_regex](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L579) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [misc:swallow_regex](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L578) | String | `STRVAL_EMPTY` | validated text/picker; dedicated builder for specialized syntax |
| [misc:vrr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L567) | Int | `0` | integer control; enumeration picker when a value map exists |
| [opengl:nvidia_anti_flicker](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L646) | Bool | `true` | switch |
| [quirks:prefer_hdr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L832) | Int | `0` | integer control; enumeration picker when a value map exists |
| [quirks:skip_non_kms_dmabuf_formats](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L833) | Bool | `true` | switch |
| [render:async_commit](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L662) | Bool | `false` | switch |
| [render:cm_auto_hdr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L659) | Int | `1` | integer control; enumeration picker when a value map exists |
| [render:cm_enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L657) | Bool | `true` | switch |
| [render:cm_sdr_eotf](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L664) | String | `"default"` | validated text/picker; dedicated builder for specialized syntax |
| [render:commit_timing_enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L665) | Bool | `true` | switch |
| [render:ctm_animation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L655) | Int | `2` | integer control; enumeration picker when a value map exists |
| [render:direct_scanout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L652) | Int | `0` | integer control; enumeration picker when a value map exists |
| [render:expand_undersized_textures](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L653) | Bool | `true` | switch |
| [render:fp16_sdr_tf](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L673) | Int | `0` | integer control; enumeration picker when a value map exists |
| [render:icc_vcgt_enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L666) | Bool | `true` | switch |
| [render:keep_unmodified_copy](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L669) | Int | `2` | integer control; enumeration picker when a value map exists |
| [render:new_render_scheduling](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L661) | Bool | `false` | switch |
| [render:non_shader_cm](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L663) | Int | `3` | integer control; enumeration picker when a value map exists |
| [render:non_shader_cm_interop](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L671) | Int | `2` | integer control; enumeration picker when a value map exists |
| [render:not_shown_fifo_lock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L674) | Int | `0` | integer control; enumeration picker when a value map exists |
| [render:send_content_type](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L658) | Bool | `true` | switch |
| [render:use_fp16](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L668) | Int | `2` | integer control; enumeration picker when a value map exists |
| [render:use_shader_blur_blend](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L667) | Bool | `false` | switch |
| [render:xp_mode](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L654) | Bool | `false` | switch |
| [scrolling:column_width](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L805) | Float | `0.5` | decimal control |
| [scrolling:direction](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L812) | String | `"right"` | validated text/picker; dedicated builder for specialized syntax |
| [scrolling:explicit_column_widths](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L811) | String | `"0.333, 0.5, 0.667, 1.0"` | validated text/picker; dedicated builder for specialized syntax |
| [scrolling:focus_fit_method](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L806) | Int | `1` | integer control; enumeration picker when a value map exists |
| [scrolling:follow_focus](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L808) | Bool | `true` | switch |
| [scrolling:follow_min_visible](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L809) | Float | `0.4` | decimal control |
| [scrolling:fullscreen_on_one_column](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L803) | Bool | `true` | switch |
| [scrolling:wrap_focus](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L813) | Bool | `true` | switch |
| [scrolling:wrap_swapcol](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L814) | Bool | `true` | switch |
| [xwayland:create_abstract_socket](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L640) | Bool | `false` | switch |
| [xwayland:enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L637) | Bool | `true` | switch |
| [xwayland:force_zero_scaling](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L639) | Bool | `false` | switch |
| [xwayland:use_nearest_neighbor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/values/ConfigValues.cpp#L638) | Bool | `true` | switch |

Current registered API additions: `hl.dsp.reload_config`, `hl.get_devices`.

Current API removals: none.

### Current structured descriptor fields

#### monitor — 22 fields

| Field | Type | Constructor/default expression |
|---|---|---|
| [mode](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L82) | String | `"preferred"` |
| [position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L84) | String | `"auto"` |
| [scale](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L86) | String | `"auto"` |
| [reserved](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L88) | CssGap | `0` |
| [reserved_area](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L93) | CssGap | `0` |
| [disabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L98) | Bool | `false` |
| [transform](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L103) | Int | `0, 0, 7` |
| [mirror](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L108) | String | `STRVAL_EMPTY` |
| [bitdepth](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L113) | Int | `8` |
| [cm](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L118) | String | `"srgb"` |
| [sdr_eotf](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L120) | String | `"default"` |
| [sdrbrightness](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L125) | Float | `1.F` |
| [sdrsaturation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L130) | Float | `1.F` |
| [vrr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L135) | Int | `-1, -1, 3` |
| [icc](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L141) | String | `STRVAL_EMPTY` |
| [supports_wide_color](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L143) | Int | `0, -1, 1` |
| [supports_hdr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L148) | Int | `0, -1, 1` |
| [sdr_min_luminance](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L153) | Float | `0.2F` |
| [sdr_max_luminance](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L158) | Int | `80` |
| [min_luminance](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L163) | Float | `-1.F` |
| [max_luminance](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L168) | Int | `-1` |
| [max_avg_luminance](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L173) | Int | `-1` |

Added: none. Removed: none.

#### device — 43 fields

| Field | Type | Constructor/default expression |
|---|---|---|
| [sensitivity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L231) | Float | `0.F, -1.F, 1.F` |
| [accel_profile](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L232) | String | `STRVAL_EMPTY` |
| [rotation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L233) | Int | `0, 0, 359` |
| [kb_file](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L234) | String | `STRVAL_EMPTY` |
| [kb_layout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L235) | String | `"us"` |
| [kb_variant](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L236) | String | `STRVAL_EMPTY` |
| [kb_options](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L237) | String | `STRVAL_EMPTY` |
| [kb_rules](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L238) | String | `STRVAL_EMPTY` |
| [kb_model](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L239) | String | `STRVAL_EMPTY` |
| [repeat_rate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L240) | Int | `25, 0, 200` |
| [repeat_delay](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L241) | Int | `600, 0, 2000` |
| [natural_scroll](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L242) | Bool | `false` |
| [tap_button_map](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L243) | String | `STRVAL_EMPTY` |
| [numlock_by_default](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L244) | Bool | `false` |
| [resolve_binds_by_sym](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L245) | Bool | `false` |
| [disable_while_typing](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L246) | Bool | `true` |
| [clickfinger_behavior](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L247) | Bool | `false` |
| [middle_button_emulation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L248) | Bool | `false` |
| [tap_to_click](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L249) | Bool | `true` |
| [tap_and_drag](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L250) | Bool | `true` |
| [drag_lock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L251) | Int | `0, 0, 2` |
| [left_handed](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L252) | Bool | `false` |
| [scroll_method](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L253) | String | `STRVAL_EMPTY` |
| [scroll_button](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L254) | Int | `0, 0, 300` |
| [scroll_button_lock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L255) | Bool | `false` |
| [scroll_points](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L256) | String | `STRVAL_EMPTY` |
| [scroll_factor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L257) | Float | `1.F, 0.F, 100.F` |
| [transform](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L258) | Int | `-1` |
| [output](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L259) | String | `STRVAL_EMPTY` |
| [enabled](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L260) | Bool | `true` |
| [region_position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L261) | Vec2 | `{0, 0}` |
| [absolute_region_position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L262) | Bool | `false` |
| [region_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L263) | Vec2 | `{0, 0}` |
| [relative_input](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L264) | Bool | `false` |
| [active_area_position](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L265) | Vec2 | `{0, 0}` |
| [active_area_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L266) | Vec2 | `{0, 0}` |
| [flip_x](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L267) | Bool | `false` |
| [flip_y](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L268) | Bool | `false` |
| [drag_3fg](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L269) | Int | `0, 0, 2` |
| [keybinds](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L270) | Bool | `true` |
| [share_states](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L271) | Int | `0, 0, 2` |
| [release_pressed_on_close](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L272) | Bool | `false` |
| [tags](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L273) | String | `STRVAL_EMPTY` |

Added: none. Removed: none.

#### workspace_rule — 15 fields

| Field | Type | Constructor/default expression |
|---|---|---|
| [monitor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L198) | String | `STRVAL_EMPTY` |
| [default](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L200) | Bool | `false` |
| [persistent](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L202) | Bool | `false` |
| [gaps_in](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L204) | CssGap | `5` |
| [gaps_out](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L206) | CssGap | `20` |
| [float_gaps](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L208) | CssGap | `0` |
| [border_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L210) | Int | `-1` |
| [no_border](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L212) | Bool | `false` |
| [no_rounding](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L214) | Bool | `false` |
| [decorate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L216) | Bool | `true` |
| [no_shadow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L218) | Bool | `false` |
| [on_created_empty](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L220) | String | `STRVAL_EMPTY` |
| [default_name](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L222) | String | `STRVAL_EMPTY` |
| [layout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L224) | String | `STRVAL_EMPTY` |
| [animation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L226) | String | `STRVAL_EMPTY` |

Added: none. Removed: none.

#### window_rule_effect — 60 fields

| Field | Type | Constructor/default expression |
|---|---|---|
| [float](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L50) | Bool | `false` |
| [tile](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L51) | Bool | `false` |
| [fullscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L52) | Bool | `false` |
| [maximize](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L53) | Bool | `false` |
| [center](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L54) | Bool | `false` |
| [pseudo](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L55) | Bool | `false` |
| [no_initial_focus](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L56) | Bool | `false` |
| [pin](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L57) | Bool | `false` |
| [fullscreen_state](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L58) | String | `STRVAL_EMPTY` |
| [move](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L59) | ExpressionVec2 | `` |
| [size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L60) | ExpressionVec2 | `` |
| [monitor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L61) | String | `STRVAL_EMPTY` |
| [workspace](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L62) | String | `STRVAL_EMPTY` |
| [group](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L63) | String | `STRVAL_EMPTY` |
| [suppress_event](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L64) | String | `STRVAL_EMPTY` |
| [content](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L65) | String | `STRVAL_EMPTY` |
| [no_close_for](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L66) | Int | `0` |
| [scrolling_width](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L67) | Float | `0.F` |
| [rounding](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L68) | Int | `0, 0, 100` |
| [border_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L69) | Int | `0` |
| [rounding_power](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L70) | Float | `2.F, 1.F, 10.F` |
| [scroll_mouse](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L71) | Float | `1.F, 0.01F, 10.F` |
| [scroll_touchpad](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L72) | Float | `1.F, 0.01F, 10.F` |
| [animation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L73) | String | `STRVAL_EMPTY` |
| [idle_inhibit](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L74) | String | `STRVAL_EMPTY` |
| [opacity](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L75) | String | `STRVAL_EMPTY` |
| [tag](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L76) | String | `STRVAL_EMPTY` |
| [max_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L77) | ExpressionVec2 | `` |
| [min_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L78) | ExpressionVec2 | `` |
| [border_color](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L79) | Gradient | `CHyprColor(0xFF000000)` |
| [persistent_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L80) | Bool | `false` |
| [allows_input](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L81) | Bool | `false` |
| [dim_around](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L82) | Bool | `false` |
| [decorate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L83) | Bool | `true` |
| [focus_on_activate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L84) | Bool | `false` |
| [keep_aspect_ratio](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L85) | Bool | `false` |
| [nearest_neighbor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L86) | Bool | `false` |
| [no_anim](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L87) | Bool | `false` |
| [no_blur](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L88) | Bool | `false` |
| [no_dim](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L89) | Bool | `false` |
| [no_focus](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L90) | Bool | `false` |
| [no_follow_mouse](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L91) | Bool | `false` |
| [no_max_size](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L92) | Bool | `false` |
| [no_shadow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L93) | Bool | `false` |
| [no_glow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L94) | Bool | `false` |
| [no_wobble](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L95) | Bool | `false` |
| [no_shortcuts_inhibit](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L96) | Bool | `false` |
| [opaque](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L97) | Bool | `false` |
| [force_rgbx](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L98) | Bool | `false` |
| [sync_fullscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L99) | Bool | `false` |
| [immediate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L100) | Bool | `false` |
| [xray](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L101) | Bool | `false` |
| [render_unfocused](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L102) | Bool | `false` |
| [no_screen_share](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L103) | Bool | `false` |
| [no_vrr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L104) | Bool | `false` |
| [no_auto_hdr](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L105) | Bool | `false` |
| [stay_focused](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L106) | Bool | `false` |
| [confine_pointer](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L107) | Bool | `false` |
| [no_xdg_drags](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L108) | Bool | `false` |
| [tonemap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsInternal.hpp#L109) | String | `STRVAL_EMPTY` |

Added: `no_glow`, `no_wobble`, `no_xdg_drags`. Removed: none.

#### layer_rule_effect — 10 fields

| Field | Type | Constructor/default expression |
|---|---|---|
| [no_anim](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L183) | Bool | `false` |
| [blur](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L184) | Bool | `false` |
| [blur_popups](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L185) | Bool | `false` |
| [ignore_alpha](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L186) | Float | `0.F, 0.F, 1.F` |
| [dim_around](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L187) | Bool | `false` |
| [xray](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L188) | Bool | `false` |
| [animation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L189) | String | `STRVAL_EMPTY` |
| [order](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L190) | Int | `0` |
| [above_lock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L191) | Int | `0, 0, 2` |
| [no_screen_share](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L192) | Bool | `false` |

Added: none. Removed: none.

Current Lua events: `config.props_refreshed`, `config.reloaded`, `config.unload`, `hyprland.shutdown`, `hyprland.start`, `input.keyboard.key`, `keybinds.submap`, `layer.closed`, `layer.opened`, `monitor.added`, `monitor.focused`, `monitor.layout_changed`, `monitor.removed`, `screenshare.state`, `window.active`, `window.bell`, `window.class`, `window.close`, `window.destroy`, `window.fullscreen`, `window.kill`, `window.minimize`, `window.move_to_workspace`, `window.open`, `window.open_early`, `window.pin`, `window.title`, `window.update_rules`, `window.urgent`, `workspace.active`, `workspace.created`, `workspace.move_to_monitor`, `workspace.removed`, `workspace.special_active`.

Current `HL.Device` methods: none. Fields: `address`, `name`, `type`.

Current `HL.EventSubscription` methods: `is_active`, `remove`. Fields: none.

Current `HL.Group` methods: `add`, `remove`. Fields: `current`, `current_index`, `denied`, `locked`, `members`, `size`.

Current `HL.Keybind` methods: `is_enabled`, `remove`, `set_enabled`, `unbind`. Fields: `allow_input_capture`, `arg`, `auto_consuming`, `catchall`, `click`, `description`, `device_inclusive`, `devices`, `display_key`, `dont_inhibit`, `drag`, `enabled`, `handler`, `has_description`, `ignore_mods`, `key`, `keycode`, `locked`, `long_press`, `modmask`, `mouse`, `non_consuming`, `release`, `repeating`, `submap`, `submap_universal`, `transparent`.

Current `HL.LayerRule` methods: `is_enabled`, `set_enabled`. Fields: none.

Current `HL.LayerSurface` methods: none. Fields: `above_fullscreen`, `address`, `h`, `keyboard_interactivity`, `layer`, `mapped`, `monitor`, `namespace`, `pid`, `w`, `x`, `y`.

Current `HL.Monitor` methods: `set_special_workspace`, `set_workspace`. Fields: `active_special_workspace`, `active_workspace`, `available_modes`, `cm`, `description`, `dpms_status`, `enabled`, `focused`, `hardware_details`, `height`, `id`, `is_mirror`, `mirrors`, `mode`, `name`, `physical_height`, `physical_width`, `position`, `refresh_rate`, `reserved`, `scale`, `serial`, `size`, `transform`, `vrr_active`, `width`, `x`, `y`.

Current `HL.Notification` methods: `dismiss`, `get_color`, `get_elapsed`, `get_elapsed_since_creation`, `get_font_size`, `get_icon`, `get_text`, `get_timeout`, `is_alive`, `is_paused`, `pause`, `resume`, `set_color`, `set_font_size`, `set_icon`, `set_paused`, `set_text`, `set_timeout`. Fields: none.

Current `HL.Timer` methods: `is_enabled`, `set_enabled`, `set_timeout`. Fields: none.

Current `HL.Window` methods: none. Fields: `accepts_input`, `active`, `address`, `allowed_over_fullscreen`, `at`, `class`, `content_type`, `floating`, `focus_history_id`, `fullscreen`, `fullscreen_client`, `fullscreen_handler`, `group`, `hidden`, `inhibiting_idle`, `initial_class`, `initial_title`, `layout`, `mapped`, `monitor`, `pid`, `pin_fullscreened`, `pinned`, `size`, `stable_id`, `swallowing`, `tags`, `tearing_hint`, `title`, `visible`, `workspace`, `xdg_description`, `xdg_tag`, `xwayland`.

Current `HL.WindowRule` methods: `is_enabled`, `set_enabled`. Fields: none.

Current `HL.Workspace` methods: `get_groups`, `get_windows`. Fields: `active`, `addressable_name`, `fullscreen_mode`, `fullscreen_window`, `groups`, `has_fullscreen`, `has_urgent`, `id`, `is_empty`, `is_persistent`, `last_window`, `monitor`, `name`, `special`, `tiled_layout`, `visible`, `windows`.

Current `HL.WorkspaceRule` methods: `is_enabled`, `set_enabled`. Fields: none.

Callable namespace aliases: none. These are additional call syntax, not new independent functions.

## Legacy compatibility surfaces in 0.56.2

Registered directives: `exec`, `execr`, `exec-once`, `execr-once`, `exec-shutdown`, `monitor`, `bind` (suffix flags), `unbind`, `workspace`, `windowrule`, `layerrule`, `bezier`, `animation`, `source`, `submap`, `plugin`, `permission`, `gesture` (suffix flags), `env` (suffix flags), `windowrulev2`, `layerrulev2`.

The 0.56.2 source has 71 legacy dispatcher names: `exec`, `execr`, `killactive`, `forcekillactive`, `closewindow`, `killwindow`, `signal`, `signalwindow`, `togglefloating`, `setfloating`, `settiled`, `workspace`, `renameworkspace`, `fullscreen`, `fullscreenstate`, `movetoworkspace`, `movetoworkspacesilent`, `pseudo`, `movefocus`, `movewindow`, `swapwindow`, `centerwindow`, `togglegroup`, `changegroupactive`, `movegroupwindow`, `focusmonitor`, `movecursortocorner`, `movecursor`, `workspaceopt`, `exit`, `movecurrentworkspacetomonitor`, `focusworkspaceoncurrentmonitor`, `moveworkspacetomonitor`, `togglespecialworkspace`, `forcerendererreload`, `resizeactive`, `moveactive`, `cyclenext`, `focuswindowbyclass`, `focuswindow`, `tagwindow`, `toggleswallow`, `submap`, `pass`, `sendshortcut`, `sendkeystate`, `layoutmsg`, `dpms`, `movewindowpixel`, `resizewindowpixel`, `swapnext`, `swapactiveworkspaces`, `pin`, `mouse`, `bringactivetotop`, `alterzorder`, `focusurgentorlast`, `focuscurrentorlast`, `lockgroups`, `lockactivegroup`, `moveintogroup`, `moveoutofgroup`, `movewindoworgroup`, `moveintoorcreategroup`, `setignoregrouplock`, `denywindowfromgroup`, `event`, `global`, `setprop`, `forceidle`, `releaseinputcapture`.

These require parser-specific builders/migrations. The current git source no longer contains the legacy implementation; do not offer unsupported legacy editing as an apply option on such builds.

## Socket2 event names in current documentation

`workspace`, `workspacev2`, `focusedmon`, `focusedmonv2`, `activewindow`, `activewindowv2`, `fullscreen`, `monitorremoved`, `monitorremovedv2`, `monitoradded`, `monitoraddedv2`, `createworkspace`, `createworkspacev2`, `destroyworkspace`, `destroyworkspacev2`, `moveworkspace`, `moveworkspacev2`, `renameworkspace`, `activespecial`, `activespecialv2`, `activelayout`, `openwindow`, `closewindow`, `kill`, `movewindow`, `movewindowv2`, `openlayer`, `closelayer`, `submap`, `changefloatingmode`, `urgent`, `screencast`, `screencastv2`, `windowtitle`, `windowtitlev2`, `togglegroup`, `moveintogroup`, `moveoutofgroup`, `ignoregrouplock`, `lockgroups`, `configreloaded`, `pin`, `minimized`, `bell`.

Lua event subscriptions and socket2 events are different interfaces with different payloads. These documented socket2 entries must be version-gated against source emissions.

## All current core documentation table entries

This supplement retains documented options, argument forms, selector forms and enum entries, including features newer than 0.56.2. These names are not all scalar keys. Section labels preserve context; a source schema must resolve nesting, aliases, defaults and accepted syntax.

### Lua events

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Events | [config.props_refreshed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [config.reloaded](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [config.unload](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [hyprland.shutdown](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [hyprland.start](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [input.keyboard.key](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [keybinds.submap](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [layer.closed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [layer.opened](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [monitor.added](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [monitor.focused](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [monitor.layout_changed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [monitor.removed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [screenshare.state](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.bell](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.class](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.close](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.destroy](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.kill](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.minimize](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.move_to_workspace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.open](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.open_early](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.pin](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.title](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.update_rules](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [window.urgent](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [workspace.active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [workspace.created](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [workspace.move_to_monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [workspace.removed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |
| Events | [workspace.special_active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/events.md) |  |  |

### Notifications

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Parameters | [text](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) | string | [[Empty]] |
| Parameters | [timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) | int | [[Required]] |
| Parameters | [icon](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) | string | "none" |
| Parameters | [color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) | color | 0 |
| Parameters | [font_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) | int | 13 |
| Icon | [None](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) |  |  |
| Icon | [Warning](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) |  |  |
| Icon | [Info](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) |  |  |
| Icon | [Hint](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) |  |  |
| Icon | [Error](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) |  |  |
| Icon | [Confused](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) |  |  |
| Icon | [Ok](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/advanced-configuration/notifications.md) |  |  |

### Mouse

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Movement | [window.drag()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/devices/mouse.md) |  |  |
| Movement | [window.resize()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/devices/mouse.md) |  |  |
| Movement | [window.resize({ keep_aspect_ratio })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/devices/mouse.md) |  |  |

### Flags

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Bind flags | [locked](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [release](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [click](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [drag](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [long_press](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [repeating](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [non_consuming](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [auto_consuming](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [mouse](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [transparent](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [ignore_mods](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [description](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [dont_inhibit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [submap_universal](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [device](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |
| Bind flags | [allow_input_capture](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/flags.md) |  |  |

### Gestures

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Fields | [action](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | string |  |
| Fields | [direction](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | string |  |
| Fields | [fingers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | int | [2 - 9] |
| Fields | [disable_inhibit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | bool |  |
| Fields | [mods](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | string |  |
| Fields | [scale](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | float | [0.1 - 10.0] |
| Directions | [swipe](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [horizontal](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [vertical](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [left](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [right](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [up](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [down](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [pinch](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [pinchin](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Directions | [pinchout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [_lua function_](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [close](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [cursor_zoom](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [float](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [move](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [resize](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [scroll_move](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [special](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Actions | [workspace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) |  |  |
| Live Lua gestures | [delta.x](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | float |  |
| Live Lua gestures | [delta.y](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | float |  |
| Live Lua gestures | [fingers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | int |  |
| Live Lua gestures | [rotation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | float |  |
| Live Lua gestures | [scale](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | float |  |
| Live Lua gestures | [time_ms](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | int |  |
| Live Lua gestures | [type](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | string |  |
| Live Lua gestures | [cancelled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | bool |  |
| Live Lua gestures | [time_ms](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | int |  |
| Live Lua gestures | [type](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/binds/gestures.md) | string |  |

### Config options

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Syntax | [string](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | [data type](https://wiki.hypr.land/Configuring/Variables/#data-types) | Default |
| General | [allow_tearing](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| General | [border_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| General | [extend_border_grab_area](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 15 |
| General | [float_gaps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | css_gaps | 0 |
| General | [gaps_in](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | css_gaps | 5 |
| General | [gaps_out](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | css_gaps | 20 |
| General | [gaps_workspaces](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| General | [hover_icon_on_border](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| General | [layout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | "dwindle" |
| General | [locale](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| General | [modal_parent_blocking](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| General | [no_focus_fallback](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| General | [resize_corner](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| General | [resize_on_border](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| General colors | [active_border](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0xffffffff |
| General colors | [inactive_border](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0xff444444 |
| General colors | [nogroup_border](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0xffffaaff |
| General colors | [nogroup_border_active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0xffff00ff |
| Snap | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Snap | [border_overlap](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Snap | [respect_gaps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Snap | [monitor_gap](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 10 |
| Snap | [window_gap](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 10 |
| Decoration | [active_opacity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Decoration | [border_part_of_window](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Decoration | [dim_around](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.4 |
| Decoration | [dim_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Decoration | [dim_modal](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Decoration | [dim_special](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.2  |
| Decoration | [dim_strength](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.5 |
| Decoration | [fullscreen_opacity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Decoration | [inactive_opacity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Decoration | [rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Decoration | [rounding_power](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 2.0 |
| Decoration | [screen_shader](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Blur | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Blur | [brightness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1 |
| Blur | [contrast](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.8916 |
| Blur | [ignore_opacity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Blur | [input_methods](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Blur | [input_methods_ignorealpha](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.2 |
| Blur | [new_optimizations](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Blur | [noise](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.0117 |
| Blur | [passes](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Blur | [popups](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Blur | [popups_ignorealpha](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.2 |
| Blur | [size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 8 |
| Blur | [special](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Blur | [variant](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | kawase |
| Blur | [vibrancy](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.1696 |
| Blur | [vibrancy_darkness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.0 |
| Blur | [xray](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Acrylic | [aberration](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.025 |
| Acrylic | [bulb](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 48.0 |
| Acrylic | [clarity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.82 |
| Acrylic | [refraction](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 24.0 |
| Acrylic | [tint](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | 0x14EEF5FF |
| Aurora | [color1](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | 0x29F0A0FF |
| Aurora | [color2](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | 0x7A4DFFFF |
| Aurora | [intensity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.35 |
| Aurora | [speed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Drops | [speed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 3.0 |
| Fluid jar | [color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | 0xCC3399FF |
| Fluid jar | [distortion](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 8.0 |
| Fluid jar | [fill_amount](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.5 |
| Fluid jar | [mass](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.4 |
| Fluid jar | [precision](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 2.0 |
| Fluid jar | [speed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 3.7 |
| Fluid jar | [turbulence](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.2 |
| Glass | [refraction](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 20.0 |
| Glass | [roughness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Glass | [size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 40.0 |
| Haze | [intensity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.35 |
| Haze | [iridescence](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.7 |
| Heat shimmer | [speed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Ripple | [duration](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.45 |
| Ripple | [radius](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 400.0 |
| Ripple | [strength](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 30.0 |
| Ripple | [width](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 32.0 |
| Water | [damping](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.95 |
| Water | [duration](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 12.0 |
| Water | [radius](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 20.0 |
| Water | [speed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.76 |
| Water | [strength](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 32.0 |
| Shadow | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Shadow | [color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color/gradient | 0xee1a1a1a |
| Shadow | [color_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color/gradient | [[Empty]] |
| Shadow | [offset](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | vec2 | {0, 0} |
| Shadow | [range](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 4 |
| Shadow | [render_power](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 3 |
| Shadow | [scale](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Shadow | [sharp](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Glow | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Glow | [color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | 0xee1a1a1a |
| Glow | [color_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | [[Empty]] |
| Glow | [range](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 10 |
| Glow | [render_power](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 3 |
| Motion blur | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Motion blur | [samples](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 7 |
| Wobble | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Wobble | [mesh](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 12 |
| Wobble | [stiffness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 200 |
| Wobble | [damping](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 12 |
| Wobble | [mass](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1 |
| Wobble | [intensity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.2 |
| Wobble | [value_epsilon](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.25 |
| Wobble | [velocity_epsilon](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 2 |
| Animations | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Animations | [workspace_wraparound](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input | [accel_profile](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Input | [emulate_discrete_scroll](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Input | [float_switch_override_focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Input | [focus_on_close](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Input | [follow_mouse](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Input | [follow_mouse_shrink](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Input | [follow_mouse_threshold](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.0 |
| Input | [force_no_accel](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input | [kb_file](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Input | [kb_layout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | "us" |
| Input | [kb_model](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Input | [kb_options](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Input | [kb_rules](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Input | [kb_variant](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Input | [left_handed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input | [mouse_refocus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Input | [natural_scroll](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input | [numlock_by_default](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input | [off_window_axis_events](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Input | [repeat_delay](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 600 |
| Input | [repeat_rate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 25 |
| Input | [resolve_binds_by_sym](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input | [rotation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Input | [scroll_button](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Input | [scroll_button_lock](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input | [scroll_points](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Input | [scroll_method](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Input | [scroll_factor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Input | [sensitivity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.0 |
| Input | [special_fallthrough](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Touchpad | [clickfinger_behavior](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Touchpad | [disable_while_typing](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Touchpad | [drag_3fg](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Touchpad | [drag_lock](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Touchpad | [flip_x](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Touchpad | [flip_y](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Touchpad | [middle_button_emulation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Touchpad | [natural_scroll](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Touchpad | [scroll_factor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Touchpad | [tap_and_drag](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Touchpad | [tap_button_map](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Touchpad | [tap_to_click](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Touchdevice | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Touchdevice | [output](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Auto]] |
| Touchdevice | [transform](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Virtualkeyboard | [release_pressed_on_close](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Virtualkeyboard | [share_states](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Tablet | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Tablet | [output](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Tablet | [transform](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Tablet | [absolute_region_position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Tablet | [active_area_position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | vec2 | {0, 0} |
| Tablet | [active_area_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | vec2 | {0, 0} |
| Tablet | [left_handed](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Tablet | [region_position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | vec2 | {0, 0} |
| Tablet | [region_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | vec2 | {0, 0} |
| Tablet | [relative_input](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Tablettool | [eraser_button_mode](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Tablettool | [eraser_button_override](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Tablettool | [pressure_range_min](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | -1.0 |
| Tablettool | [pressure_range_max](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | -1.0 |
| Gestures | [close_max_timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1000 |
| Gestures | [workspace_swipe_cancel_ratio](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.5 |
| Gestures | [workspace_swipe_create_new](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Gestures | [workspace_swipe_direction_lock](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Gestures | [workspace_swipe_direction_lock_threshold](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 10 |
| Gestures | [workspace_swipe_distance](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 300 |
| Gestures | [workspace_swipe_forever](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Gestures | [workspace_swipe_invert](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Gestures | [workspace_swipe_min_speed_to_force](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 30 |
| Gestures | [workspace_swipe_touch](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Gestures | [workspace_swipe_touch_invert](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Gestures | [workspace_swipe_use_r](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Scrolling | [move_snap_to_grid](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Scrolling | [move_snap_cursor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Group | [auto_group](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Group | [drag_into_group](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Group | [focus_removed_window](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Group | [group_on_movetoworkspace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Group | [insert_after_current](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Group | [merge_floated_into_tiled_on_groupbar](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Group | [merge_groups_on_drag](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Group | [merge_groups_on_groupbar](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Group colors | [border_active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0x66ffff00 |
| Group colors | [border_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0x66777700 |
| Group colors | [border_locked_active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0x66ff5500 |
| Group colors | [border_locked_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0x66775500 |
| Groupbar | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Groupbar | [blur](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Groupbar | [disable_when_only](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Groupbar | [font_family](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Groupbar | [font_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 8 |
| Groupbar | [font_weight_active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | font_weight | "normal" |
| Groupbar | [font_weight_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | font_weight | "normal" |
| Groupbar | [gaps_in](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Groupbar | [gaps_out](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Groupbar | [gradients](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Groupbar | [gradient_round_only_edges](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Groupbar | [gradient_rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Groupbar | [gradient_rounding_power](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 2.0 |
| Groupbar | [height](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 14 |
| Groupbar | [indicator_gap](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Groupbar | [indicator_height](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 3 |
| Groupbar | [keep_upper_gap](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Groupbar | [middle_click_close](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Groupbar | [priority](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 3 |
| Groupbar | [render_titles](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Groupbar | [round_only_edges](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Groupbar | [rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Groupbar | [rounding_power](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 2.0 |
| Groupbar | [scrolling](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Groupbar | [stacked](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Groupbar | [text_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | 0xffffffff |
| Groupbar | [text_color_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | [[Empty]] |
| Groupbar | [text_color_locked_active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | [[Empty]\ |
| Groupbar | [text_color_locked_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | [[Empty]\ |
| Groupbar | [text_offset](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Groupbar | [text_padding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Groupbar colors | [active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0x66ffff00 |
| Groupbar colors | [inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0x66777700 |
| Groupbar colors | [locked_active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0x66ff5500 |
| Groupbar colors | [locked_inactive](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | gradient | 0x66775500 |
| Misc | [allow_session_lock_restore](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [always_follow_on_dnd](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Misc | [animate_manual_resizes](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [animate_mouse_windowdragging](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [anr_missed_pings](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 5 |
| Misc | [background_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | 0x111111 |
| Misc | [bell_sound](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | "default" |
| Misc | [close_special_on_empty](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Misc | [disable_autoreload](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [disable_hyprland_guiutils_check](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [disable_hyprland_logo](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [disable_scale_notification](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [disable_splash_rendering](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [disable_watchdog_warning](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [disable_xdg_env_checks](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [enable_anr_dialog](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Misc | [enable_swallow](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [exit_window_retains_fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Misc | [float_force_onscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Misc | [new_float_force_onscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Misc | [focus_on_activate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [font_family](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | "Sans" |
| Misc | [force_default_wallpaper](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | -1 |
| Misc | [initial_workspace_token_timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 10 |
| Misc | [initial_workspace_tracking](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Misc | [key_press_enables_dpms](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [layers_hog_keyboard_focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Misc | [lockdead_screen_delay](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1000 |
| Misc | [middle_click_paste](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Misc | [mouse_move_enables_dpms](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [mouse_move_focuses_monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Misc | [name_vk_after_proc](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Misc | [on_focus_under_fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Misc | [render_unfocused_fps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 15 |
| Misc | [screencopy_force_8b](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Misc | [session_lock_blur](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [session_lock_xray](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [size_limits_tiled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Misc | [splash_font_family](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Misc | [swallow_exception_regex](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Misc | [swallow_regex](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Misc | [vrr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Misc | [splash](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | color | 0x55ffffff |
| Layout | [single_window_aspect_ratio](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | vec2 | {0, 0} |
| Layout | [single_window_aspect_ratio_tolerance](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0.1 |
| Binds | [allow_pin_fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Binds | [allow_workspace_cycles](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Binds | [disable_keybind_grabbing](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Binds | [drag_threshold](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Binds | [focus_preferred_method](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Binds | [hide_special_on_workspace_change](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Binds | [ignore_group_lock](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Binds | [movefocus_cycles_fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Binds | [movefocus_cycles_groupfirst](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Binds | [pass_mouse_when_bound](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Binds | [scroll_event_delay](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 300 |
| Binds | [window_direction_monitor_fallback](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Binds | [workspace_back_and_forth](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Binds | [workspace_center_on](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Xwayland | [enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Xwayland | [create_abstract_socket](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Xwayland | [force_zero_scaling](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Xwayland | [use_nearest_neighbor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| OpenGL | [nvidia_anti_flicker](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Render | [async_commit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Render | [cm_auto_hdr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Render | [cm_enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Render | [cm_sdr_eotf](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | "default" |
| Render | [ctm_animation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Render | [commit_timing_enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Render | [direct_scanout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Render | [expand_undersized_textures](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Render | [fp16_sdr_tf](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Render | [icc_vcgt_enabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Render | [keep_unmodified_copy](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Render | [new_render_scheduling](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Render | [not_shown_fifo_lock](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Render | [non_shader_cm](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Render | [non_shader_cm_interop](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Render | [send_content_type](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Render | [use_fp16](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Render | [use_shader_blur_blend](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Render | [xp_mode](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Cursor | [default_monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | string | [[Empty]] |
| Cursor | [enable_hyprcursor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Cursor | [hide_on_key_press](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Cursor | [hide_on_tablet](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Cursor | [hide_on_touch](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Cursor | [hotspot_padding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Cursor | [inactive_timeout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 0 |
| Cursor | [invisible](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Cursor | [min_refresh_rate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 24 |
| Cursor | [no_break_fs_vrr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Cursor | [no_hardware_cursors](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Cursor | [no_warps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Cursor | [persistent_warps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Cursor | [sync_gsettings_theme](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Cursor | [use_cpu_buffer](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Cursor | [warp_back_after_non_mouse_input](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Cursor | [warp_on_change_workspace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Cursor | [warp_on_monitor_change](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | -1 |
| Cursor | [warp_on_toggle_special](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Cursor | [zoom_detached_camera](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Cursor | [zoom_disable_aa](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Cursor | [zoom_factor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | float | 1.0 |
| Cursor | [zoom_rigid](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Ecosystem | [enforce_permissions](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Ecosystem | [no_donation_nag](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Ecosystem | [no_update_news](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Quirks | [prefer_hdr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Quirks | [skip_non_kms_dmabuf_formats](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input Capture | [capture_modifiers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Input Capture | [enforce_barriers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Debug | [colored_stdout_logs](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Debug | [damage_blink](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [damage_tracking](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 2 |
| Debug | [disable_logs](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Debug | [disable_scale_checks](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [disable_time](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Debug | [ds_handle_same_buffer](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Debug | [ds_handle_same_buffer_fifo](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Debug | [enable_stdout_logs](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [error_limit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 5 |
| Debug | [error_position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Debug | [fifo_pending_workaround](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [full_cm_proto](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [gl_debugging](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [invalidate_buffers](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 1 |
| Debug | [log_damage](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [manual_crash](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | int | 0 |
| Debug | [overlay](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [pass](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [render_solitary_wo_damage](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [suppress_errors](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |
| Debug | [vfr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | true |
| Experimental | [wp_cm_1_2](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/config-options.md) | bool | false |

### Dispatchers

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Parameter explanation | [window](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Parameter explanation | [action](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Parameter explanation | [relative](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [dpms({ monitor?, action? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [event( string )](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [exec_cmd( cmd, { rules }? )](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [exec_raw( cmd )](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [exit()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [focus({ direction })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [focus({ monitor })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [focus({ workspace, on_current_monitor? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [focus({ window })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [focus({ urgent_or_last })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [focus({ last })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [force_idle( int )](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [force_renderer_reload()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [global( string )](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [layout( message )](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [no_op()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [pass({ window? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [release_input_capture()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [reload_config()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [send_key_state({ window?, mods, key, state })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [send_shortcut({ window?, mods, key })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| General | [submap( name )](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [alter_zorder({ window?, mode })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [center({ window? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [clear_tags({ window? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [close({ window? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [cycle_next({ window?, next?, tiled?, floating? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [deny_from_group({ action? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [drag()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [float({ window?, action? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [fullscreen_state({ window?, action?, internal, client, layout_aware? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [fullscreen({ window?, action?, mode?, layout_aware? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [kill({ window? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [move({ window?, direction, group_aware? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [move({ window?, workspace, follow? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [move({ window?, monitor, follow? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [move({ window?, x, y, relative? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [move({ window?, into_group = direction })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [move({ window?, into_or_create_group = direction })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [move({ window?, out_of_group })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [pin({ window?, action? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [pseudo({ window?, action? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [resize({ keep_aspect_ratio? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [resize({ window?, x, y, relative? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [set_prop({ window?, prop, value })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [signal({ window?, signal })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [swap({ direction })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [swap({ target })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [swap({ next })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [swap({ prev })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [tag({ window?, tag })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Window | [toggle_swallow()](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Workspace | [move({ workspace?, monitor })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Workspace | [rename({ workspace, name? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Workspace | [swap_monitors({ monitor1, monitor2 })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Workspace | [toggle_special( special_name )](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Group | [active({ window?, index })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Group | [lock({ window?, action? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Group | [lock_active({ action? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Group | [move_window({ window?, forward? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Group | [next({ window? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Group | [prev({ window? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Group | [toggle({ window? })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Cursor | [move({ x, y })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| Cursor | [move_to_corner({ window?, corner })](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| fullscreen_state | [-1](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| fullscreen_state | [0](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| fullscreen_state | [1](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |
| fullscreen_state | [2](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/dispatchers.md) |  |  |

### Monitors

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Fields | [output](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | string | [[Required]] |
| Fields | [disabled](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | bool | false |
| Fields | [mode](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | string | "preferred" |
| Fields | [scale](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | float/string | "auto" |
| Fields | [transform](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | int | 0 |
| Fields | [position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | string | "auto" |
| Fields | [mirror](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | string | [[Empty]] |
| Fields | [bitdepth](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | int | 8 |
| Fields | [cm](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | string | "srgb" |
| Fields | [sdr_eotf](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | string | "default" |
| Fields | [vrr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | int | 0 |
| Fields | [icc](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | string | [[Empty]] |
| Fields | [reserved_area](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | css_gaps | 0 |
| Fields | [supports_wide_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | int | 0 |
| Fields | [supports_hdr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | int | 0 |
| Fields | [sdrbrightness](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | float | 1.0 |
| Fields | [sdrsaturation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | float | 1.0 |
| Fields | [sdr_min_luminance](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | float | 0.2 |
| Fields | [sdr_max_luminance](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | int | 80 |
| Fields | [min_luminance](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | float | -1 |
| Fields | [max_luminance](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | int | -1 |
| Fields | [max_avg_luminance](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/monitors/_index.md) | int | -1 |

### Layer rules

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Props | [namespace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | [RegEx] |  |
| Effects | [above_lock](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | int |  |
| Effects | [animation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | string |  |
| Effects | [blur](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | bool |  |
| Effects | [blur_popups](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | bool |  |
| Effects | [dim_around](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | bool |  |
| Effects | [ignore_alpha](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | float | [0.0 - 1.0] |
| Effects | [no_anim](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | bool |  |
| Effects | [no_screen_share](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | bool |  |
| Effects | [order](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | int |  |
| Effects | [xray](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/layer-rules.md) | bool |  |

### Window Rules

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Props | [class](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | [RegEx] |  |
| Props | [content](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Props | [focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Props | [fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Props | [fullscreen_state_client](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | int |  |
| Props | [fullscreen_state_internal](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | int |  |
| Props | [float](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Props | [group](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Props | [initial_class](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | [RegEx] |  |
| Props | [initial_title](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | [RegEx] |  |
| Props | [modal](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Props | [pin](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Props | [tag](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Props | [title](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | [RegEx] |  |
| Props | [workspace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | workspace selector |  |
| Props | [xdg_tag](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | [RegEx] |  |
| Props | [xwayland](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [center](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [content](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Static effects | [float](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [fullscreen_state](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Static effects | [group](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Static effects | [maximize](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Static effects | [move](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | table or string |  |
| Static effects | [no_close_for](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | int |  |
| Static effects | [no_initial_focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [pin](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [pseudo](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [scrolling_width](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | float |  |
| Static effects | [size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | table or string |  |
| Static effects | [suppress_event](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Static effects | [tile](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Static effects | [workspace](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Dynamic effects | [allows_input](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [animation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Dynamic effects | [border_color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | gradient |  |
| Dynamic effects | [border_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | int |  |
| Dynamic effects | [confine_pointer](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [dim_around](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [decorate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [focus_on_activate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [idle_inhibit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Dynamic effects | [immediate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [keep_aspect_ratio](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [max_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | table or string |  |
| Dynamic effects | [min_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | table or string |  |
| Dynamic effects | [nearest_neighbor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_anim](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_auto_hdr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_blur](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_dim](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_follow_mouse](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_glow](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_max_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_screen_share](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_shadow](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_shortcuts_inhibit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_vrr](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_wobble](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [no_xdg_drags](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [opacity](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Dynamic effects | [opaque](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [force_rgbx](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [persistent_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [render_unfocused](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | int |  |
| Dynamic effects | [rounding_power](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | float |  |
| Dynamic effects | [scroll_mouse](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | float |  |
| Dynamic effects | [scroll_touchpad](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | float |  |
| Dynamic effects | [stay_focused](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [sync_fullscreen](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |
| Dynamic effects | [tag](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Dynamic effects | [tonemap](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | string |  |
| Dynamic effects | [xray](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/window-rules.md) | bool |  |

### Workspace Rules

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Rules | [animation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | string |  |
| Rules | [border_size](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | int |  |
| Rules | [decorate](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | bool |  |
| Rules | [default_name](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | string |  |
| Rules | [float_gaps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | css_gaps |  |
| Rules | [gaps_in](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | css_gaps |  |
| Rules | [gaps_out](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | css_gaps |  |
| Rules | [layout](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | string |  |
| Rules | [layout_opts](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | table |  |
| Rules | [monitor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | string |  |
| Rules | [default](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | bool |  |
| Rules | [no_border](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | bool |  |
| Rules | [no_rounding](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | bool |  |
| Rules | [no_shadow](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | bool |  |
| Rules | [on_created_empty](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | string |  |
| Rules | [persistent](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/core/rules/workspace-rules.md) | bool |  |

### Dwindle layout

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Config | [force_split](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | int | 0 |
| Config | [preserve_split](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | bool | false |
| Config | [smart_split](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | bool | false |
| Config | [smart_resizing](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | bool | true |
| Config | [permanent_direction_override](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | bool | false |
| Config | [special_scale_factor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | float | 1 |
| Config | [split_width_multiplier](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | float | 1.0 |
| Config | [use_active_for_splits](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | bool | true |
| Config | [default_split_ratio](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | float | 1.0 |
| Config | [split_bias](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | int | 0 |
| Config | [precise_mouse_move](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | bool | false |
| Dispatchers | [window.pseudo](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) | Left empty/`"active"` for current, or `"window"` for a specific window |  |
| Layout messages | [movetoroot](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) |  |  |
| Layout messages | [preselect](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) |  |  |
| Layout messages | [rotatesplit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) |  |  |
| Layout messages | [splitratio](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) |  |  |
| Layout messages | [swapsplit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) |  |  |
| Layout messages | [togglesplit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/dwindle-layout.md) |  |  |

### Master layout

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Config | [allow_small_split](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | bool | false |
| Config | [special_scale_factor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | float | 1 |
| Config | [mfact](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | float | 0.55 |
| Config | [new_status](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | string | "slave" |
| Config | [new_on_top](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | bool | false |
| Config | [new_on_active](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | string | "none" |
| Config | [orientation](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | string | "left" |
| Config | [slave_count_for_center_master](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | int | 2 |
| Config | [center_master_fallback](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | string | "left" |
| Config | [smart_resizing](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | bool | true |
| Config | [drop_at_cursor](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | bool | true |
| Config | [always_keep_position](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | bool | false |
| Config | [focus_master_on_close](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | bool | false |
| Layout messages | [addmaster](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [cyclenext](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | either `loop` (allow looping from the bottom of the pile back to master) or `noloop` (force stop at the bottom of the pile, like in DWM). `loop` is the default if left blank. |  |
| Layout messages | [cycleprev](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | either `loop` (allow looping from master to the bottom of the pile) or `noloop` (force stop at master, like in DWM). `loop` is the default if left blank. |  |
| Layout messages | [focusmaster](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | either `master` (focus stays on master), `auto` (default; focus first non-master window if already on master) or `previous` (remember current window when focusing master, if already on master, focus previous or fallback to `auto`). |  |
| Layout messages | [mfact](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | the new split ratio, a relative float delta (e.g., `-0.2`, `+0.2`) or `exact` followed by a value between 0.0 and 1.0 |  |
| Layout messages | [orientationbottom](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [orientationcenter](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [orientationcycle](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | allowed values: `left`, `top`, `right`, `bottom`, or `center`. The values have to be separated by a space. If left empty, it will work like `orientationnext |  |
| Layout messages | [orientationleft](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [orientationnext](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [orientationprev](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [orientationright](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [orientationtop](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [removemaster](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [rollnext](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [rollprev](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | none |  |
| Layout messages | [swapnext](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | either `loop` (allow swapping the bottom of the pile and master) or `noloop` (do not allow it, like in DWM). `loop` is the default if left blank. |  |
| Layout messages | [swapprev](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | either `loop` (allow swapping master and the bottom of the pile) or `noloop` (do not allow it, like in DWM). `loop` is the default if left blank. |  |
| Layout messages | [swapwithmaster](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | either `master` (new focus is the new master window), `child` (new focus is the new child) or `auto` (which is the default, keeps the focus of the previously focused window). Adding `ignoremaster` will ignore this dispatcher if master is already focused. |  |
| Workspace Rules | [orientation = [o]](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/master-layout.md) | string |  |

### Monocle layout

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Layout messages | [cyclenext](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/monocle-layout.md) | none |  |
| Layout messages | [cycleprev](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/monocle-layout.md) | none |  |

### Scrolling layout

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Config | [fullscreen_on_one_column](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | bool | true |
| Config | [column_width](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | float | 0.5 |
| Config | [focus_fit_method](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | int | 1 |
| Config | [follow_focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | bool | true |
| Config | [follow_min_visible](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | float | 0.4 |
| Config | [explicit_column_widths](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | string | "0.333, 0.5, 0.667, 1.0" |
| Config | [wrap_focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | bool | true |
| Config | [wrap_swapcol](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | bool | true |
| Config | [direction](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | string | "right" |
| Workspace rules | [direction](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | string |  |
| Layout messages | [center](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | none |  |
| Layout messages | [colresize](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | relative float/relative `conf |  |
| Layout messages | [consume](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | none |  |
| Layout messages | [consume_or_expel](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | prev` or `next |  |
| Layout messages | [expel](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | none |  |
| Layout messages | [fit](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | fit mode |  |
| Layout messages | [fit_into_view](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | none |  |
| Layout messages | [focus](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | direction |  |
| Layout messages | [inhibit_scroll](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | left empty for toggle, or `bool` for explicitly enabling/disabling |  |
| Layout messages | [move](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | move data |  |
| Layout messages | [promote](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | none |  |
| Layout messages | [swapcol](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/layouts/scrolling-layout.md) | l` or `r |  |

### Naming conventions

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Data types | [int](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | int |  |
| Data types | [float](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | float |  |
| Data types | [bool](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | bool |  |
| Data types | [string](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | string |  |
| Data types | [table](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | table |  |
| Data types | [vec2](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | vec2 |  |
| Data types | [css_gaps](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | css_gaps |  |
| Data types | [color](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | color |  |
| Data types | [gradient](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | gradient |  |
| Data types | [font_weight](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/naming-conventions.md) | font_weight |  |

### Selectors

| Section | Documented field or form | Type/argument | Default/limits |
|---|---|---|---|
| Workspace query | [e](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/selectors.md) |  | Wraps around if range exceeds amount of worksapces in the direction |
| Workspace query | [m](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/selectors.md) |  | Wraps around if range exceeds amount of worksapces in the direction |
| Workspace query | [r](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/selectors.md) |  | [1 - ...] |
| Workspace query | [empty](https://github.com/hyprwm/hyprland-wiki/blob/fde84d41b9a7154a5d211cfb1de50d5910791231/content/configuring/selectors.md) |  | Wraps around if it lands past the last numerical workspace, 2147483647 |


## Literal socket2 emission names from source

**0.56.2:** `activelayout`, `activespecial`, `activespecialv2`, `activewindow`, `activewindowv2`, `bell`, `changefloatingmode`, `changeworkspaceid`, `closelayer`, `closewindow`, `configreloaded`, `createworkspace`, `createworkspacev2`, `custom`, `destroyworkspace`, `destroyworkspacev2`, `focusedmon`, `focusedmonv2`, `fullscreen`, `kill`, `lockgroups`, `minimized`, `monitoradded`, `monitoraddedv2`, `monitorremoved`, `monitorremovedv2`, `moveintogroup`, `moveoutofgroup`, `movewindow`, `movewindowv2`, `moveworkspace`, `moveworkspacev2`, `openlayer`, `openwindow`, `pin`, `renameworkspace`, `screencast`, `screencastv2`, `submap`, `togglegroup`, `urgent`, `windowtitle`, `windowtitlev2`, `workspace`, `workspacev2`.

**Current git:** `activelayout`, `activespecial`, `activespecialv2`, `activewindow`, `activewindowv2`, `bell`, `changefloatingmode`, `changeworkspaceid`, `closelayer`, `closewindow`, `configreloaded`, `createworkspace`, `createworkspacev2`, `custom`, `destroyworkspace`, `destroyworkspacev2`, `focusedmon`, `focusedmonv2`, `fullscreen`, `kill`, `lockgroups`, `minimized`, `monitoradded`, `monitoraddedv2`, `monitorremoved`, `monitorremovedv2`, `moveintogroup`, `moveoutofgroup`, `movewindow`, `movewindowv2`, `moveworkspace`, `moveworkspacev2`, `openlayer`, `openwindow`, `pin`, `renameworkspace`, `screencast`, `screencastv2`, `submap`, `togglegroup`, `urgent`, `windowtitle`, `windowtitlev2`, `workspace`, `workspacev2`.

These include custom and implementation emissions not all listed in the docs; plugins and dynamic emitters can add arbitrary event names. Subscribe to socket2 without mistaking its payloads for Lua event payloads.
