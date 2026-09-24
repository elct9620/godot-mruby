class InheritedTest < Minitest::Test
  # @behavior RS-023
  def test_a_nodes_object_is_given_a_callback_its_class_inherits_from_another_file
    boss = Godot::Node2D.new
    boss.set_script(Godot::ResourceLoader.load("res://test/unit/script/inherited/boss.rb"))

    add_child_autofree(boss)

    assert_equal "Unit::Script::Inherited::Boss", boss.call(:readied)
  end
end
