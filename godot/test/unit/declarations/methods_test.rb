class MethodsTest < Minitest::Test
  # @behavior RD-004
  def test_a_node_has_a_method_its_class_defined_as_it_ran
    bell = autofree(Unit::Declarations::Bell.new)

    assert bell.has_method(:toll)
  end

  # @behavior RD-005
  def test_a_method_a_class_defined_as_it_ran_is_among_its_scripts_methods
    bell = autofree(Unit::Declarations::Bell.new)
    script = Godot::ResourceLoader.load("res://test/unit/declarations/bell.rb")

    names = script.get_script_method_list.map { |method| method["name"] }

    assert_includes names, "toll"
  end

  # @behavior RD-010
  def test_a_node_has_a_method_the_class_its_class_extends_defined_as_it_ran
    chime = autofree(Unit::Declarations::Chime.new)

    assert chime.has_method(:toll)
  end

  # @behavior RD-006
  def test_a_callback_a_class_defined_as_it_ran_is_called_on_the_node
    bell = autofree(Unit::Declarations::Bell.new)

    bell.notification(9001)

    assert_equal 9001, bell.last_notification
  end

  # @behavior RD-069
  def test_a_node_has_a_method_a_module_its_class_takes_in_defines
    carillon = autofree(Unit::Declarations::Carillon.new)

    assert carillon.has_method(:strike)
  end

  # @behavior RD-070
  def test_a_callback_a_module_its_class_takes_in_defines_is_called_on_the_node
    carillon = autofree(Unit::Declarations::Carillon.new)

    carillon.notification(9002)

    assert_equal 9002, carillon.last_notification
  end

  # @behavior RD-071
  def test_a_node_has_a_method_its_class_gains_after_its_file_ran
    belfry = autofree(Unit::Declarations::Belfry.new)

    Unit::Declarations::Belfry.define_method(:knell) { "tolled" }

    assert belfry.has_method(:knell)
  end

  # @behavior RD-072
  def test_a_node_has_a_method_of_a_module_its_class_takes_in_after_its_file_ran
    belfry = autofree(Unit::Declarations::Belfry.new)

    Unit::Declarations::Belfry.include(Unit::Declarations::Rope)

    assert belfry.has_method(:pull)
  end
end
