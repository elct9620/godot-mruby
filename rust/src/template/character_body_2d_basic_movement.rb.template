class _CLASS_ < _BASE_
_TS_SPEED = 300.0
_TS_JUMP_VELOCITY = -400.0

_TS_def _physics_process(delta)
_TS__TS_# Add the gravity.
_TS__TS_self.velocity += get_gravity * delta unless is_on_floor

_TS__TS_# Handle jump.
_TS__TS_if Godot::Input.is_action_just_pressed("ui_accept") && is_on_floor
_TS__TS__TS_self.velocity = Godot::Vector2.new(velocity.x, JUMP_VELOCITY)
_TS__TS_end

_TS__TS_# Get the input direction and handle the movement/deceleration.
_TS__TS_# As good practice, you should replace UI actions with custom gameplay actions.
_TS__TS_direction = Godot::Input.get_axis("ui_left", "ui_right")
_TS__TS_x = direction.zero? ? Godot.move_toward(velocity.x, 0, SPEED) : direction * SPEED
_TS__TS_self.velocity = Godot::Vector2.new(x, velocity.y)

_TS__TS_move_and_slide
_TS_end
end
