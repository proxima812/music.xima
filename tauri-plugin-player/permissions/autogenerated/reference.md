## Default Permission

Full access to the native player: playback, queue editing, library scanning and
the storage roots the user picked. The plugin is private to music.xima and the
whole surface is needed by the app shell, so the default set allows every
command from `build.rs` (CONTRACTS §7).

#### This default permission set includes the following:

- `allow-get-state`
- `allow-set-queue`
- `allow-play`
- `allow-pause`
- `allow-toggle`
- `allow-stop`
- `allow-next`
- `allow-previous`
- `allow-seek`
- `allow-skip-to`
- `allow-set-shuffle`
- `allow-set-repeat`
- `allow-set-volume`
- `allow-set-speed`
- `allow-add-next`
- `allow-add-to-queue`
- `allow-remove-queue-item`
- `allow-move-queue-item`
- `allow-clear-queue`
- `allow-scan-media-store`
- `allow-scan-tree`
- `allow-pick-folder`
- `allow-persisted-roots`
- `allow-release-root`
- `allow-extract-artwork`

## Permission Table

<table>
<tr>
<th>Identifier</th>
<th>Description</th>
</tr>


<tr>
<td>

`player:allow-add-next`

</td>
<td>

Enables the add_next command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-add-next`

</td>
<td>

Denies the add_next command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-add-to-queue`

</td>
<td>

Enables the add_to_queue command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-add-to-queue`

</td>
<td>

Denies the add_to_queue command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-clear-queue`

</td>
<td>

Enables the clear_queue command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-clear-queue`

</td>
<td>

Denies the clear_queue command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-extract-artwork`

</td>
<td>

Enables the extract_artwork command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-extract-artwork`

</td>
<td>

Denies the extract_artwork command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-get-state`

</td>
<td>

Enables the get_state command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-get-state`

</td>
<td>

Denies the get_state command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-move-queue-item`

</td>
<td>

Enables the move_queue_item command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-move-queue-item`

</td>
<td>

Denies the move_queue_item command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-next`

</td>
<td>

Enables the next command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-next`

</td>
<td>

Denies the next command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-pause`

</td>
<td>

Enables the pause command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-pause`

</td>
<td>

Denies the pause command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-persisted-roots`

</td>
<td>

Enables the persisted_roots command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-persisted-roots`

</td>
<td>

Denies the persisted_roots command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-pick-folder`

</td>
<td>

Enables the pick_folder command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-pick-folder`

</td>
<td>

Denies the pick_folder command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-play`

</td>
<td>

Enables the play command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-play`

</td>
<td>

Denies the play command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-previous`

</td>
<td>

Enables the previous command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-previous`

</td>
<td>

Denies the previous command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-release-root`

</td>
<td>

Enables the release_root command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-release-root`

</td>
<td>

Denies the release_root command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-remove-queue-item`

</td>
<td>

Enables the remove_queue_item command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-remove-queue-item`

</td>
<td>

Denies the remove_queue_item command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-scan-media-store`

</td>
<td>

Enables the scan_media_store command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-scan-media-store`

</td>
<td>

Denies the scan_media_store command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-scan-tree`

</td>
<td>

Enables the scan_tree command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-scan-tree`

</td>
<td>

Denies the scan_tree command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-seek`

</td>
<td>

Enables the seek command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-seek`

</td>
<td>

Denies the seek command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-set-queue`

</td>
<td>

Enables the set_queue command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-set-queue`

</td>
<td>

Denies the set_queue command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-set-repeat`

</td>
<td>

Enables the set_repeat command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-set-repeat`

</td>
<td>

Denies the set_repeat command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-set-shuffle`

</td>
<td>

Enables the set_shuffle command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-set-shuffle`

</td>
<td>

Denies the set_shuffle command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-set-speed`

</td>
<td>

Enables the set_speed command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-set-speed`

</td>
<td>

Denies the set_speed command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-set-volume`

</td>
<td>

Enables the set_volume command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-set-volume`

</td>
<td>

Denies the set_volume command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-skip-to`

</td>
<td>

Enables the skip_to command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-skip-to`

</td>
<td>

Denies the skip_to command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-stop`

</td>
<td>

Enables the stop command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-stop`

</td>
<td>

Denies the stop command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:allow-toggle`

</td>
<td>

Enables the toggle command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`player:deny-toggle`

</td>
<td>

Denies the toggle command without any pre-configured scope.

</td>
</tr>
</table>
