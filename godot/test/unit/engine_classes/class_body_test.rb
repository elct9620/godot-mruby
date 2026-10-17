class ClassBodyTest < Minitest::Test
  # @behavior RG-005
  def test_a_class_extending_an_engine_class_calls_tool_abstract_and_icon
    marked = Class.new(Godot::Node) do
      tool
      abstract
      icon "res://icon.svg"
    end

    assert_equal Godot::Node, marked.superclass
  end

  # @behavior RG-006
  def test_an_abstract_class_cannot_be_made
    enemy = Class.new(Godot::Node) { abstract }

    assert_raises(NotImplementedError) { enemy.new }
  end

  # @behavior RG-007
  def test_a_class_extending_an_abstract_class_is_not_abstract_itself
    rabbit = autofree(Unit::EngineClasses::Rabbit.new)

    assert_instance_of Unit::EngineClasses::Rabbit, rabbit
  end

  # @behavior RG-052
  def test_a_method_abstract_marks_raises_when_called
    hare = autofree(Unit::EngineClasses::Hare.new)

    error = assert_raises(NotImplementedError) { hare.hop(1) }
    assert_equal %(Cannot call the parent class' abstract function "hop()" because it hasn't been defined.), error.message
  end

  # @behavior RG-053
  def test_a_class_writing_an_abstract_method_runs_its_own
    rabbit = autofree(Unit::EngineClasses::Rabbit.new)

    assert_equal "hops 2", rabbit.hop(2)
  end

  # @behavior RG-054
  def test_abstract_leaves_a_method_on_the_class_itself_as_it_is
    shape = Class.new(Godot::Node) do
      abstract
      abstract def self.count = 3
    end

    assert_equal [3, false], [shape.count, shape.method_defined?(:count)]
  end

  # @behavior RG-055
  def test_abstract_given_a_name_an_ancestors_class_method_has_marks_the_method
    charm = autofree(Unit::EngineClasses::Charm.new)

    assert_raises(NotImplementedError) { charm.icon }
  end
end
