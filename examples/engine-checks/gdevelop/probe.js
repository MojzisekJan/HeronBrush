// Injected into an exported build: reports the Heron level's state to the console.
(function () {
  const start = Date.now();
  const timer = setInterval(() => {
    const game = window.__heronGame;
    const scene = game && game.getSceneStack().getCurrentScene();
    if (!scene || !scene.__heronLevels) return;
    const level = Object.values(scene.__heronLevels)[0];
    const t = (Date.now() - start) / 1000;
    const ball = (scene.getObjects('ProbeBall') || [])[0];
    const step = window.__heronProbeStep || 0;
    if (step === 0 && t > 3) {
      window.__heronProbeStep = 1;
      const all = scene.getAdhocListOfAllInstances();
      const bodies = all.filter(o => o.getBehavior && o.getBehavior('Physics3D')).map(o => o.getName() + ':' + o.getBehavior('Physics3D').bodyType);
      console.log('HERON objects=' + all.length + ' bodies=' + bodies.join(' '));
      console.log('HERON ball z=' + (ball ? ball.getZ().toFixed(2) : 'none') + ' gameTime=' + scene.getTimeManager().getTimeFromStart());
      const target = window.__heronTrigger, mover = window.__heronTarget;
      if (target && ball) {
        const e = level.entities[mover].objects[0];
        window.__heronStart = [e.getX(), e.getY(), e.getZ(), e.getAngle()];
        const t0 = level.entities[target];
        ball.getVariables().get('HeronActivator').setBoolean(true);
        const b = ball.getBehavior('Physics3D');
        const pos = t0.position;
        ball.setX(pos[0] * 100 - 25); ball.setY(pos[2] * 100 - 25); ball.setZ(pos[1] * 100 + 2);
        console.log('HERON walker into ' + target + ' playing=' + level.animations[mover].playing);
      }
    }
    if (step === 1 && t > 7) {
      window.__heronProbeStep = 2;
      const mover = window.__heronTarget;
      if (mover) {
        const e = level.entities[mover].objects[0], a = level.animations[mover];
        console.log('HERON ' + mover + ' clip=' + a.clip.name + ' start=' + JSON.stringify(window.__heronStart) + ' now=' + JSON.stringify([e.getX(), e.getY(), e.getZ(), e.getAngle(), e.getRotationX(), e.getRotationY()].map(v => +v.toFixed(2))));
      }
      console.log('HERON ball z=' + (ball ? ball.getZ().toFixed(2) : 'none') + ' gameTime=' + scene.getTimeManager().getTimeFromStart()); console.log('HERON done');
      clearInterval(timer);
    }
  }, 100);
})();
// ?aim=angle,tilt points the camera after start: GDevelop's camera angle and its
// rotationX (90 looks at the horizon).
(function () {
  const m = /aim=([-\d.]+),([-\d.]+)/.exec(location.search);
  if (!m) return;
  const t = setInterval(() => {
    const game = window.__heronGame, scene = game && game.getSceneStack().getCurrentScene();
    if (!scene || !scene.__heronLevels) return;
    clearInterval(t);
    setTimeout(() => {
      gdjs.scene3d.camera.setCameraZ(scene, 2000, '', 0);
      gdjs.scene3d.camera.setCameraRotationX(scene, +m[2], '', 0);
      gdjs.evtTools.camera.setCameraRotation(scene, +m[1], '', 0);
    }, 500);
  }, 100);
})();
