extends Node3D
@export var charge: int = 0
@export var armed: bool = false
@export var sequence: Array = []
var entered := 0

func _ready() -> void:
    assert(charge == 7, "Authored value must override the project's default of 9")
    assert(armed and sequence == [2, 3, 5], "Typed project defaults were not passed")
    assert(get_meta("properties")["charge"] == 7, "Typed metadata was not preserved")

func on_puzzle_enter(body: Node3D) -> void:
    if body is CharacterBody3D:
        entered += 1
