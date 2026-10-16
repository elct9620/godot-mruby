class HintsTest < Minitest::Test
  # @GlobalScope's PropertyHint values, which Ruby has no names for.
  RANGE = 1
  ENUM = 2
  FLAGS = 6
  FILE = 13
  DIR = 14
  MULTILINE_TEXT = 18
  PLACEHOLDER_TEXT = 20
  EXP_EASING = 4
  LAYERS_2D_PHYSICS = 8
  GLOBAL_FILE = 15
  GLOBAL_DIR = 16
  COLOR_NO_ALPHA = 21
  TYPE_STRING = 23
  NODE_PATH_VALID_TYPES = 26
  LAYERS_AVOIDANCE = 37
  TOOL_BUTTON = 39
  INPUT_NAME = 43
  FILE_PATH = 44
  # @GlobalScope's PropertyUsageFlags values a script's variables carry.
  STORAGE = 2
  EDITOR = 4
  SCRIPT_VARIABLE = 4096
  READ_ONLY = 268_435_456

  # @behavior RD-039
  def test_a_range_hint_carries_the_bounds_it_was_declared_with
    angle = property_by_name("angle")

    assert_equal RANGE, angle["hint"]
    assert_equal "0,360", angle["hint_string"]
  end

  # @behavior RD-040
  def test_a_range_hint_carries_the_step_it_was_declared_with
    speed = property_by_name("speed")

    assert_equal "1,10,2", speed["hint_string"]
  end

  # @behavior RD-041
  def test_an_enum_hint_names_the_values_it_was_declared_with
    aim = property_by_name("aim")

    assert_equal ENUM, aim["hint"]
    assert_equal "nearest,strongest", aim["hint_string"]
  end

  # @behavior RD-042
  def test_a_flags_hint_names_the_flags_it_was_declared_with
    terrain = property_by_name("terrain")

    assert_equal FLAGS, terrain["hint"]
    assert_equal "water,fire", terrain["hint_string"]
  end

  # @behavior RD-043
  def test_a_file_hint_carries_the_filter_it_was_declared_with
    sound = property_by_name("sound")

    assert_equal FILE, sound["hint"]
    assert_equal "*.ogg", sound["hint_string"]
  end

  # @behavior RD-044
  def test_a_directory_hint_is_declared_without_a_filter
    folder = property_by_name("folder")

    assert_equal DIR, folder["hint"]
    assert_equal "", folder["hint_string"]
  end

  # @behavior RD-045
  def test_a_multiline_hint_is_declared_without_a_filter
    notes = property_by_name("notes")

    assert_equal MULTILINE_TEXT, notes["hint"]
    assert_equal "", notes["hint_string"]
  end

  # @behavior RD-046
  def test_a_placeholder_hint_carries_the_text_it_was_declared_with
    title = property_by_name("title")

    assert_equal PLACEHOLDER_TEXT, title["hint"]
    assert_equal "Name", title["hint_string"]
  end

  # @behavior RD-047
  def test_a_hint_the_exported_type_cannot_take_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Sign }

    assert_equal %("range:" requires a variable of type "int" or "float", but type "String" was given instead.),
                 error.message
  end

  # @behavior RD-048
  def test_a_range_leaving_out_its_end_is_refused
    assert_raises(ArgumentError) { Unit::Declarations::Ramp }
  end

  # @behavior RD-049
  def test_a_step_without_a_range_is_refused
    assert_raises(ArgumentError) { Unit::Declarations::Stair }
  end

  # @behavior RD-050
  def test_a_second_hint_on_one_property_is_refused
    assert_raises(ArgumentError) { Unit::Declarations::Slider }
  end

  # @behavior RD-067
  def test_a_keyword_naming_no_hint_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Tag }

    assert_equal %("slider:" is not a hint a property can be exported with.), error.message
  end

  # @behavior RD-068
  def test_a_list_hint_written_with_anything_but_a_list_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Menu }

    assert_equal %("enum:" needs a list of names, but :one was given instead.), error.message
  end

  # @behavior RD-051
  def test_a_nodes_properties_carry_the_hints_its_class_declared
    gauge = autofree(Unit::Declarations::Gauge.new)

    angle = gauge.get_property_list.find { |property| property["name"] == "angle" }

    assert_equal RANGE, angle["hint"]
    assert_equal "0,360", angle["hint_string"]
  end

  # @behavior RD-091
  def test_a_color_no_alpha_hint_is_declared_without_a_filter
    assert_equal [COLOR_NO_ALPHA, ""], hint_of(console_property("tint"))
  end

  # @behavior RD-092
  def test_an_exp_easing_hint_names_the_options_it_was_declared_with
    assert_equal [EXP_EASING, "attenuation,positive_only"], hint_of(console_property("fade"))
  end

  # @behavior RD-093
  def test_a_file_path_hint_carries_the_filter_it_was_declared_with
    assert_equal [FILE_PATH, "*.png"], hint_of(console_property("icon_path"))
  end

  # @behavior RD-094
  def test_a_global_file_hint_carries_the_filter_it_was_declared_with
    assert_equal [GLOBAL_FILE, "*.log"], hint_of(console_property("log_file"))
  end

  # @behavior RD-095
  def test_a_global_directory_hint_is_declared_without_a_filter
    assert_equal [GLOBAL_DIR, ""], hint_of(console_property("save_dir"))
  end

  # @behavior RD-096
  def test_a_layer_hint_is_the_layers_its_keyword_names
    assert_equal [LAYERS_2D_PHYSICS, LAYERS_AVOIDANCE],
                 [console_property("hit_layers")["hint"], console_property("avoid_layers")["hint"]]
  end

  # @behavior RD-097
  def test_a_node_path_hint_names_the_node_classes_it_was_declared_with
    assert_equal [NODE_PATH_VALID_TYPES, "Button,Label"], hint_of(console_property("target"))
  end

  # @behavior RD-098
  def test_a_storage_property_is_stored_and_not_shown
    assert_equal STORAGE | SCRIPT_VARIABLE, console_property("secret")["usage"]
  end

  # @behavior RD-099
  def test_a_tool_button_is_shown_with_its_text_and_not_stored
    reset = console_property("reset")

    assert_equal [TOOL_BUTTON, "Reset", EDITOR | SCRIPT_VARIABLE], [reset["hint"], reset["hint_string"], reset["usage"]]
  end

  # @behavior RD-100
  def test_a_tool_buttons_property_calls_the_objects_method
    console = autofree(Unit::Declarations::Console.new)

    assert_equal "restarted", console.get(:reset).call
  end

  # @behavior RD-101
  def test_a_custom_hint_carries_the_hint_text_and_usage_it_was_declared_with
    action = console_property("action")

    assert_equal [INPUT_NAME, "show_builtin", 6 | READ_ONLY | SCRIPT_VARIABLE],
                 [action["hint"], action["hint_string"], action["usage"]]
  end

  # @behavior RD-102
  def test_a_range_hint_carries_the_extra_hints_it_was_declared_with
    assert_equal "0,100,1,or_greater,suffix:dB", console_property("volume")["hint_string"]
  end

  # @behavior RD-103
  def test_a_typed_array_names_its_elements_type
    assert_equal [[TYPE_STRING, "2:"], [TYPE_STRING, "24/17:Texture2D"]],
                 [hint_of(console_property("scores")), hint_of(console_property("sprites"))]
  end

  # @behavior RD-104
  def test_a_typed_dictionary_names_its_keys_and_values_types
    assert_equal [TYPE_STRING, "4:;2:"], hint_of(console_property("prices"))
  end

  # @behavior RD-105
  def test_an_objects_typed_container_is_typed_as_declared
    console = autofree(Unit::Declarations::Console.new)

    assert_equal [true, 2, true, 4, 2],
                 [console.scores.is_typed, console.scores.get_typed_builtin,
                  console.prices.is_typed, console.prices.get_typed_key_builtin, console.prices.get_typed_value_builtin]
  end

  # @behavior RD-106
  def test_a_ruby_array_written_to_a_typed_property_is_held_typed
    console = autofree(Unit::Declarations::Console.new)

    console.scores = [1, 2]

    assert_equal [true, 2], [console.scores.is_typed, console.scores.get_typed_builtin]
  end

  # @behavior RD-107
  def test_a_typed_container_declared_with_another_value_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Bin }

    assert_equal %(Cannot assign a value of type "int" as "Array[int]".), error.message
  end

  private

  # The property of that name among the ones the gauge's script exported.
  def property_by_name(name)
    Unit::Declarations::Gauge
    script = Godot::ResourceLoader.load("res://test/unit/declarations/gauge.rb")
    script.get_script_property_list.find { |property| property["name"] == name }
  end

  # The property of that name among the ones the console's script exported.
  def console_property(name)
    Unit::Declarations::Console
    script = Godot::ResourceLoader.load("res://test/unit/declarations/console.rb")
    script.get_script_property_list.find { |property| property["name"] == name }
  end

  def hint_of(property)
    [property["hint"], property["hint_string"]]
  end
end
