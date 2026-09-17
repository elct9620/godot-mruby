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
    enemy = Class.new(Godot::Node) { abstract }
    boss = Class.new(enemy)

    assert_instance_of boss, boss.new
  end
end
