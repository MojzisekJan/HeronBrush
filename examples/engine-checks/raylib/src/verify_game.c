// Common-demo game checks using the production systems, without a renderer.
#include "game.h"
#include "raymath.h"
#include <math.h>
#include <stdio.h>
#include <string.h>

static int failures;
static void check(bool ok, const char *label) {
    printf("%s %s\n", ok ? "PASS" : "FAIL", label);
    if (!ok) failures++;
}

int main(void) {
    static HeronBrushLevel level;
    static HeronBrushGame game;
    if (!HeronBrushLoadLevel("assets/maps/demo.level", &level)) return 1;
    HeronBrushCollider collider;
    HeronBrushColliderBuild(&collider, &level);
    HeronBrushMovers movers;
    HeronBrushMoversBuild(&movers, &collider, &level);
    int portals = 0;
    for (int i = 0; i < level.entityCount; i++) {
        const HeronBrushEntity *source = &level.entities[i];
        if (strcmp(source->classname, "fps_portal") || !source->targetCount) continue;
        const HeronBrushEntity *destination = HeronBrushFindEntity(&level, source->targets[0]);
        check(destination != NULL, "portal resolves destination");
        if (!destination) continue;
        Vector3 feet = {source->pos[0], source->pos[1], source->pos[2]};
        HeronBrushPlayer player;
        HeronBrushPlayerInit(&player, feet, 0.4f);
        player.pos.y = source->pos[1] + 1;
        HeronBrushGameInit(&game, &level, feet, 0.4f);
        game.state = HERON_BRUSH_PLAYING;
        HeronBrushGameUpdate(&game, &level, &collider, &player, &movers, 1.0f / 60);
        float yaw = destination->rot[1];
        Vector3 expected = {destination->pos[0] - sinf(yaw) * 1.6f,
            destination->pos[1] + HERON_BRUSH_PLAYER_HALF, destination->pos[2] - cosf(yaw) * 1.6f};
        check(Vector3Distance(player.pos, expected) < 0.0001f, "portal places player at exported destination");
        check(fabsf(player.yaw - yaw) < 0.0001f, "portal faces destination yaw");
        Vector3 arrival = player.pos;
        for (int step = 0; step < 30; step++)
            HeronBrushGameUpdate(&game, &level, &collider, &player, &movers, 1.0f / 60);
        check(Vector3Distance(player.pos, arrival) < 0.0001f, "portal cooldown prevents immediate return");
        portals++;
    }
    check(portals >= 2, "both common-demo portal mouths checked");
    int doors = 0;
    for (int i = 0; i < movers.count; i++) {
        HeronBrushMover *door = &movers.items[i];
        if (strcmp(level.entities[door->entIndex].classname, "func_door")) continue;
        Vector3 centre = Vector3Scale(Vector3Add(door->restMin, door->restMax), 0.5f);
        int openSteps = (int)((1 / door->rate + 1) * 60);
        for (int step = 0; step < openSteps; step++)
            HeronBrushMoversUpdate(&movers, &collider, &level, centre, centre, 0.35f, NULL, 0, 1.0f / 60);
        check(door->amount > 0.99f, "common-demo door opens on approach");
        Vector3 minimum = {0}, maximum = {0};
        check(HeronBrushColliderEntityBounds(&collider, door->entIndex, &minimum, &maximum), "door retains exact collider");
        Vector3 offset = Vector3Scale(door->openDir, door->openDist);
        check(Vector3Distance(minimum, Vector3Add(door->restMin, offset)) < 0.001f,
              "door collider follows open pose");
        Vector3 away = {100, 100, 100};
        int closeSteps = (int)((door->wait + 1 / door->rate + 1) * 60);
        for (int step = 0; step < closeSteps; step++)
            HeronBrushMoversUpdate(&movers, &collider, &level, away, away, 0.35f, NULL, 0, 1.0f / 60);
        check(door->amount < 0.01f, "common-demo door closes after wait");
        HeronBrushColliderEntityBounds(&collider, door->entIndex, &minimum, &maximum);
        check(Vector3Distance(minimum, door->restMin) < 0.001f, "door collider returns to closed pose");
        doors++;
    }
    check(doors >= 3, "all common-demo ordinary doors checked");
    HeronBrushMoversFree(&movers);
    HeronBrushColliderFree(&collider);
    if (!failures) puts("HERON_RAYLIB_GAMEPLAY_OK");
    return failures ? 2 : 0;
}
