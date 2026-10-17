class DynamicPropertiesTest < Minitest::Test
  # @behavior RD-109
  def test_godot_reads_a_property_the_class_answers_through_get
    chest = autofree(Unit::Declarations::Chest.new)
    chest.__send__(:_set, :slot_1, 5)

    assert_equal 5, chest.get(:slot_1)
  end

  # @behavior RD-110
  def test_godot_writes_a_property_the_class_takes_through_set
    chest = autofree(Unit::Declarations::Chest.new)

    chest.set(:slot_2, 7)

    assert_equal 7, chest._get(:slot_2)
    assert_equal 7, chest.get(:slot_2)
  end

  # @behavior RD-111
  def test_a_node_lists_the_properties_its_class_lists
    names = autofree(Unit::Declarations::Chest.new).get_property_list.map { |property| property["name"] }

    assert_includes names, "slot_1"
  end

  # @behavior RD-112
  def test_a_node_lists_a_property_as_its_class_validates_it
    listed = autofree(Unit::Declarations::Chest.new).get_property_list
    description = listed.find { |property| property["name"] == "editor_description" }

    assert_equal 0, description["usage"]
  end

  # @behavior RD-113
  def test_a_node_prints_as_its_class_answers_to_s
    chest = autofree(Unit::Declarations::Chest.new)
    chest.set(:slot_1, 1)

    assert_equal "chest of 1", chest.to_string
  end
end
