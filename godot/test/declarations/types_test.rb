class TypesTest < Minitest::Test
  # @GlobalScope's PropertyHint and Variant.Type values, which Ruby has no
  # names for.
  RESOURCE_TYPE = 17
  NODE_TYPE = 34
  OBJECT = 24

  # @behavior RD-052
  def test_an_export_naming_a_node_class_is_read_as_a_node_of_it
    spot = exported("spot")

    assert_equal OBJECT, spot["type"]
    assert_equal NODE_TYPE, spot["hint"]
    assert_equal "Marker2D", spot["hint_string"]
  end

  # @behavior RD-053
  def test_an_export_naming_a_resource_class_is_read_as_a_resource_of_it
    level = exported("level")

    assert_equal RESOURCE_TYPE, level["hint"]
    assert_equal "PackedScene", level["hint_string"]
  end

  # @behavior RD-054
  def test_an_export_naming_a_ruby_class_is_read_by_the_name_it_was_announced_under
    guard = exported("guard")

    assert_equal NODE_TYPE, guard["hint"]
    assert_equal "Turret", guard["hint_string"]
  end

  # @behavior RD-055
  def test_an_export_naming_a_type_takes_no_value_of_another_type
    error = assert_raises(ArgumentError) { Loader::Misvalued }

    assert_equal %(Cannot assign a value of type int to variable "spot" with specified type Marker2D.),
                 error.message
  end

  # @behavior RD-056
  def test_an_export_naming_neither_a_node_nor_a_resource_class_is_refused
    error = assert_raises(ArgumentError) { Loader::Countless }

    assert_equal "Export type can only be built-in, a resource, a node, or an enum.", error.message
  end

  # @behavior RD-057
  def test_an_export_naming_a_class_no_announcement_lists_is_refused
    error = assert_raises(ArgumentError) { Loader::Unlisted }

    assert_equal %(The class "Loader::Items::Potion" was not found in the global scope.), error.message
  end

  private

  # The property of that name among the ones the aiming class exported.
  def exported(name)
    Loader::Aiming
    script = Godot::ResourceLoader.load("res://loader/aiming.rb")
    script.get_script_property_list.find { |property| property["name"] == name }
  end
end
