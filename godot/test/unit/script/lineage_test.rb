class LineageTest < Minitest::Test
  DIRECTORY = "res://test/unit/script/lineage".freeze

  # @behavior RS-023
  def test_a_nodes_object_is_given_a_callback_its_class_inherits_from_another_file
    boss = Godot::Node2D.new
    boss.set_script(load_script("boss"))

    add_child_autofree(boss)

    assert_equal "Unit::Script::Lineage::Boss", boss.call(:ready_class_name)
  end

  # @behavior RS-074
  def test_a_node_whose_class_inherits_from_a_scripts_class_is_taken_where_that_script_is_the_type
    boss = autofree(Godot::Node2D.new)
    boss.set_script(load_script("boss"))

    array = put_into_typed_array(boss, load_script("enemy"))

    assert_equal 1, array.size
  end

  # @behavior RS-075
  def test_a_node_whose_class_does_not_inherit_from_a_scripts_class_is_refused_where_that_script_is_the_type
    enemy = autofree(Godot::Node2D.new)
    enemy.set_script(load_script("enemy"))

    array = put_into_typed_array(enemy, load_script("boss"))

    assert_empty array
  end

  private

  def load_script(name)
    Godot::ResourceLoader.load("#{DIRECTORY}/#{name}.rb")
  end

  # What an array typed by `script` holds once `node` is put into it, as
  # Godot checks each object a typed array takes against its script.
  def put_into_typed_array(node, script)
    expression = Godot::Expression.new
    expression.parse("Array([node], typeof(node), \"Node2D\", script)", ["node", "script"])
    expression.execute([node, script])
  end
end
