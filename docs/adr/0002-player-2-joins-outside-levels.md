# Player 2 joins only outside levels, but either player can drop out anywhere

Play is single player by default, even with a second input device connected. A second player joins by pressing Start (or Enter) on an input device that is not Player 1's, and only on the title screen or an overworld map. Either player can drop out at any time from the pause menu their own device opened. This follows Lego Star Wars: The Complete Saga, except for joining: there, Player 2 can also join in the middle of a level. We don't allow that because Player 2's in-level state is built when a level is entered, and building it inside a running level would mean placing a new player, camera framing and bubble state into an already-live session.

## Considered Options

- **Always-on keyboard Player 2 (the original prototype)**: rejected for players; keyboard-only play must be single player. The fixed WASD layout stays as a debug-only option for testing.
- **Join anywhere, including mid-level (full Complete Saga behaviour)**: deferred; it needs mid-level spawning of Player 2.
- **Only Player 1 can remove Player 2**: rejected; whoever opens the pause menu owns it, and "Drop out" applies to that player only.

## Consequences

- Whether Player 2 is present is decided outside levels and stays fixed until that player drops out or the game quits; it is never saved, so every launch starts single player.
- Dropping out can happen inside a level, so a level must be able to lose a player mid-play (Player 2 leaving, or Player 1 leaving and Player 2 becoming Player 1).
