# Unity export check

Scripts that check the Unity final export and live link in a disposable project
(tested with Unity 6, Built-in Render Pipeline), using the shared demo map.

## Export and run

From the repository root:

```sh
Unity -batchmode -nographics -quit -createProject /tmp/heron-unity-check -logFile /tmp/unity-create.log
HERON_UNITY_LIVE_PROJECT=/tmp/heron-unity-check cargo test -p editor_core --lib native_unity_live_fixtures -- --ignored
python3 examples/engine-checks/unity/install_check.py /tmp/heron-unity-check Assets/FinalMap VerifyExport.cs
Unity -batchmode -nographics -projectPath /tmp/heron-unity-check -executeMethod VerifyExport.Run -logFile /tmp/unity-check.log
Unity -batchmode -projectPath /tmp/heron-unity-check -executeMethod VerifyExport.Render -logFile /tmp/unity-render.log
```

`Unity` is the editor executable; the render step needs a GPU. The checks cover
meshes, lights, colliders and triggers, live move/delete/undo, failed-update
recovery and scene save/reopen. Success markers: `HERON_UNITY_CHECK PASS` and
`HERON_UNITY_RENDER PASS`. Use a fresh project when addon sources change.

## Gameplay kit

The optional [Gameplay kit](Gameplay/README.md) adds a player and demo behaviours:

```sh
cp -r examples/engine-checks/unity/Gameplay /tmp/heron-unity-check/Assets/HeronBrushDemo
cp -r examples/engine-checks/unity/Tests /tmp/heron-unity-check/Assets/HeronBrushTests
Unity -batchmode -nographics -projectPath /tmp/heron-unity-check -executeMethod VerifyGameplay.Run -logFile /tmp/unity-gameplay.log
Unity -batchmode -nographics -quit -projectPath /tmp/heron-unity-check -executeMethod VerifyGameplay.Build -logFile /tmp/unity-player-build.log
/tmp/heron-unity-player/HeronBrushDemo.x86_64 -screen-fullscreen 0 -screen-width 960 -screen-height 640 --heron-demo-capture /tmp/unity-gameplay.png
```

Play Mode tests need an active Unity licence. `HERON_UNITY_BUILD` sets the player
output path.

## Animation and custom games

- `VerifyMotion.Run`: see [target parity](../../../docs/target-parity.md).
  `HeronBrushMotionPlayer` carries a CharacterController (feet origin, centre
  (0, 0.9, 0), height 1.8, radius 0.35, layer 31).
- `prepare_custom_game.py` then `VerifyCustom.Run` in
  `/tmp/heron-unity-custom-game`: a project-owned `Puzzle` component with typed
  fields and references.

Not covered: Windows and macOS editors, URP and HDRP, and Play Mode or Prefab Mode UI.
