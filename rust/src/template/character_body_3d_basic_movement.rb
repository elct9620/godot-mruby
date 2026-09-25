class _CLASS_ < _BASE_
_TS_SPEED = 5.0
_TS_JUMP_VELOCITY = 4.5

_TS_def _physics_process(delta)
_TS__TS_# Add the gravity.
_TS__TS_self.velocity += get_gravity * delta unless is_on_floor

_TS__TS_# Handle jump.
_TS__TS_if Godot::Input.is_action_just_pressed("ui_accept") && is_on_floor
_TS__TS__TS_self.velocity = Godot::Vector3.new(velocity.x, JUMP_VELOCITY, velocity.z)
_TS__TS_end

_TS__TS_# Get the input direction and handle the movement/deceleration.
_TS__TS_# As good practice, you should replace UI actions with custom gameplay actions.
_TS__TS_input_dir = Godot::Input.get_vector("ui_left", "ui_right", "ui_up", "ui_down")
_TS__TS_direction = (transform.basis * Godot::Vector3.new(input_dir.x, 0, input_dir.y)).normalized
_TS__TS_self.velocity =
_TS__TS__TS_if direction.is_zero_approx
_TS__TS__TS__TS_Godot::Vector3.new(Godot.move_toward(velocity.x, 0, SPEED), velocity.y, Godot.move_toward(velocity.z, 0, SPEED))
_TS__TS__TS_else
_TS__TS__TS__TS_Godot::Vector3.new(direction.x * SPEED, velocity.y, direction.z * SPEED)
_TS__TS__TS_end

_TS__TS_move_and_slide
_TS_end
end
