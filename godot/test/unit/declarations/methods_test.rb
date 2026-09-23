class MethodsTest < Minitest::Test
  # @behavior RD-004
  def test_a_node_has_a_method_its_class_defined_as_it_ran
    bell = Unit::Declarations::Bell.new

    assert bell.has_method(:toll)
  ensure
    bell&.free
  end

  # @behavior RD-005
  def test_a_method_a_class_defined_as_it_ran_is_among_its_scripts_methods
    bell = Unit::Declarations::Bell.new
    script = Godot::ResourceLoader.load("res://test/unit/declarations/bell.rb")

    names = script.get_script_method_list.map { |method| method["name"] }

    assert_includes names, "toll"
  ensure
    bell&.free
  end

  # @behavior RD-010
  def test_a_node_has_a_method_the_class_its_class_extends_defined_as_it_ran
    chime = Unit::Declarations::Chime.new

    assert chime.has_method(:toll)
  ensure
    chime&.free
  end

  # @behavior RD-006
  def test_a_callback_a_class_defined_as_it_ran_is_called_on_the_node
    bell = Unit::Declarations::Bell.new

    bell.notification(9001)

    assert_equal 9001, bell.noticed
  ensure
    bell&.free
  end
end
