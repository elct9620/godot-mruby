class RevertTest < Minitest::Test
  # @behavior RS-046
  def test_property_can_revert_tells_godot_a_property_can_be_reverted
    assert revertible.property_can_revert(:speed)
  end

  # @behavior RS-047
  def test_property_get_revert_gives_godot_the_value_a_property_reverts_to
    assert_equal 12, revertible.property_get_revert(:speed)
  end

  private

  def revertible
    node = autofree(Godot::Node.new)
    node.set_script(Godot::ResourceLoader.load("res://test/unit/script/revertible.rb"))
    node
  end
end
