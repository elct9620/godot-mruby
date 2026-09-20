class GroupsTest < Minitest::Test
  # @GlobalScope's PropertyUsageFlags, which Ruby has no names for.
  GROUP = 64
  SUBGROUP = 256
  CATEGORY = 128

  # @behavior RD-058
  def test_a_heading_is_listed_where_the_class_wrote_it
    names = listed.map { |member| member["name"] }

    assert_operator names.index("Targeting"), :<, names.index("aim_mode")
  end

  # @behavior RD-059
  def test_a_group_takes_the_properties_whose_names_begin_with_its_prefix
    targeting = listed.find { |member| member["name"] == "Targeting" }

    assert_equal GROUP, targeting["usage"]
    assert_equal "aim_", targeting["hint_string"]
  end

  # @behavior RD-060
  def test_a_subgroup_is_listed_as_a_subgroup
    look = listed.find { |member| member["name"] == "Look" }

    assert_equal SUBGROUP, look["usage"]
  end

  # @behavior RD-061
  def test_a_category_is_listed_as_a_category
    panel = listed.find { |member| member["name"] == "Panel" }

    assert_equal CATEGORY, panel["usage"]
  end

  # @behavior RD-062
  def test_a_nodes_properties_carry_the_headings_its_class_wrote
    panel = Loader::Panel.new

    targeting = panel.get_property_list.find { |member| member["name"] == "Targeting" }

    assert_equal GROUP, targeting["usage"]
  ensure
    panel&.free
  end

  private

  # What the panel's script says it has, headings and properties alike.
  def listed
    Loader::Panel
    Godot::ResourceLoader.load("res://loader/panel.rb").get_script_property_list
  end
end
