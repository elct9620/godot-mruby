class HintsTest < Minitest::Test
  # @GlobalScope's PropertyHint values, which Ruby has no names for.
  RANGE = 1
  ENUM = 2
  FLAGS = 6
  FILE = 13
  DIR = 14
  MULTILINE_TEXT = 18
  PLACEHOLDER_TEXT = 20

  # @behavior RD-039
  def test_a_range_hint_carries_the_bounds_it_was_declared_with
    angle = exported("angle")

    assert_equal RANGE, angle["hint"]
    assert_equal "0,360", angle["hint_string"]
  end

  # @behavior RD-040
  def test_a_range_hint_carries_the_step_it_was_declared_with
    speed = exported("speed")

    assert_equal "1,10,2", speed["hint_string"]
  end

  # @behavior RD-041
  def test_an_enum_hint_names_the_values_it_was_declared_with
    aim = exported("aim")

    assert_equal ENUM, aim["hint"]
    assert_equal "nearest,strongest", aim["hint_string"]
  end

  # @behavior RD-042
  def test_a_flags_hint_names_the_flags_it_was_declared_with
    terrain = exported("terrain")

    assert_equal FLAGS, terrain["hint"]
    assert_equal "water,fire", terrain["hint_string"]
  end

  # @behavior RD-043
  def test_a_file_hint_carries_the_filter_it_was_declared_with
    sound = exported("sound")

    assert_equal FILE, sound["hint"]
    assert_equal "*.ogg", sound["hint_string"]
  end

  # @behavior RD-044
  def test_a_directory_hint_is_declared_without_a_filter
    folder = exported("folder")

    assert_equal DIR, folder["hint"]
    assert_equal "", folder["hint_string"]
  end

  # @behavior RD-045
  def test_a_multiline_hint_is_declared_without_a_filter
    notes = exported("notes")

    assert_equal MULTILINE_TEXT, notes["hint"]
    assert_equal "", notes["hint_string"]
  end

  # @behavior RD-046
  def test_a_placeholder_hint_carries_the_text_it_was_declared_with
    title = exported("title")

    assert_equal PLACEHOLDER_TEXT, title["hint"]
    assert_equal "Name", title["hint_string"]
  end

  # @behavior RD-047
  def test_a_hint_the_exported_type_cannot_take_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Mistyped }

    assert_equal %("range:" requires a variable of type "int" or "float", but type "String" was given instead.),
                 error.message
  end

  # @behavior RD-048
  def test_a_range_leaving_out_its_end_is_refused
    assert_raises(ArgumentError) { Unit::Declarations::Endless }
  end

  # @behavior RD-049
  def test_a_step_without_a_range_is_refused
    assert_raises(ArgumentError) { Unit::Declarations::Stepping }
  end

  # @behavior RD-050
  def test_a_second_hint_on_one_property_is_refused
    assert_raises(ArgumentError) { Unit::Declarations::Doubling }
  end

  # @behavior RD-067
  def test_a_keyword_naming_no_hint_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Misnamed }

    assert_equal %("slider:" is not a hint a property can be exported with.), error.message
  end

  # @behavior RD-068
  def test_a_list_hint_written_with_anything_but_a_list_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Listless }

    assert_equal %("enum:" needs a list of names, but :one was given instead.), error.message
  end

  # @behavior RD-051
  def test_a_nodes_properties_carry_the_hints_its_class_declared
    gauge = autofree(Unit::Declarations::Gauge.new)

    angle = gauge.get_property_list.find { |property| property["name"] == "angle" }

    assert_equal RANGE, angle["hint"]
    assert_equal "0,360", angle["hint_string"]
  end

  private

  # The property of that name among the ones the gauge's script exported.
  def exported(name)
    Unit::Declarations::Gauge
    script = Godot::ResourceLoader.load("res://test/unit/declarations/gauge.rb")
    script.get_script_property_list.find { |property| property["name"] == name }
  end
end
