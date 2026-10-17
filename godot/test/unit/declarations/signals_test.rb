class SignalsTest < Minitest::Test
  # @GlobalScope's Variant.Type and PropertyUsageFlags, which Ruby has no
  # names for.
  NIL = 0
  BOOL = 1
  INT = 2
  VECTOR2 = 5
  OBJECT = 24
  NIL_IS_VARIANT = 131_072

  # @behavior RD-001
  def test_a_declared_signal_is_one_the_node_has
    bell = autofree(Unit::Declarations::Bell.new)

    assert bell.has_signal(:rung)
  end

  # @behavior RD-002
  def test_a_declared_signal_carries_the_parameters_it_was_declared_with
    bell = autofree(Unit::Declarations::Bell.new)

    rung = bell.get_signal_list.find { |signal| signal["name"] == "rung" }

    assert_equal ["times"], rung["args"].map { |argument| argument["name"] }
  end

  # @behavior RD-003
  def test_a_declared_signal_reaches_what_connected_to_it
    bell = autofree(Unit::Declarations::Bell.new)
    bell.connect(:rung, method(:hear))

    bell.ring(3)

    assert_equal [3], @heard
  end

  # @behavior RD-007
  def test_a_signal_declared_again_with_other_parameters_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Gong }

    assert_equal "res://test/unit/declarations/gong.rb:6", Minitest.filter_backtrace(error.backtrace).first
  end

  # @behavior RD-008
  def test_a_signal_declared_again_as_it_stands_is_no_new_signal
    repeater = autofree(Unit::Declarations::Repeater.new)

    rung = repeater.get_signal_list.select { |signal| signal["name"] == "rung" }

    assert_equal 1, rung.size
  end

  # @behavior RD-009
  def test_a_node_has_the_signal_the_class_its_class_extends_declared
    chime = autofree(Unit::Declarations::Chime.new)

    assert chime.has_signal(:rung)
  end

  # @behavior RD-084
  def test_a_signal_parameter_declared_with_a_ruby_class_is_typed_as_the_engine_type_it_converts_to
    count = herald_parameter("count")

    assert_equal INT, count["type"]
    assert_equal 0, count["usage"]
  end

  # @behavior RD-085
  def test_a_signal_parameter_of_godot_bool_is_typed_bool
    assert_equal BOOL, herald_parameter("loud")["type"]
    assert_kind_of Godot::Bool, true
    assert_kind_of Godot::Bool, false
  end

  # @behavior RD-086
  def test_a_signal_parameter_of_an_engine_class_takes_that_classs_objects
    spot = herald_parameter("spot")

    assert_equal [OBJECT, :Node2D], [spot["type"], spot["class_name"]]
  end

  # @behavior RD-087
  def test_a_signal_parameter_of_a_node_scripts_class_takes_objects_of_the_name_it_is_announced_by
    assert_equal :Enemy, herald_parameter("foe")["class_name"]
    assert_equal :Node, herald_parameter("bell")["class_name"]
  end

  # @behavior RD-088
  def test_a_signal_parameter_of_godot_variant_is_of_any_type
    payload = herald_parameter("payload")

    assert_equal [NIL, NIL_IS_VARIANT], [payload["type"], payload["usage"]]
  end

  # @behavior RD-089
  def test_a_signal_parameter_declared_with_anything_but_a_class_is_refused
    error = assert_raises(ArgumentError) { Class.new(Godot::Node) { signal :rung, times: 3 } }

    assert_equal %("times:" needs a class, but 3 was given instead.), error.message
  end

  # @behavior RD-090
  def test_a_signal_parameter_of_a_value_types_class_is_typed_as_that_value_type
    assert_equal VECTOR2, herald_parameter("at")["type"]
  end

  def hear(times)
    (@heard ||= []) << times
  end

  private

  def herald_parameter(name)
    herald = autofree(Unit::Declarations::Herald.new)
    signal = herald.get_signal_list.find { |entry| entry["name"] == "announced" }
    signal["args"].find { |argument| argument["name"] == name }
  end
end
