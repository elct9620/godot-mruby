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

  # @behavior RD-063
  def test_a_scripts_members_are_headed_by_a_category_for_its_class
    first = inherited.first

    assert_equal "cannon.rb", first["name"]
    assert_equal CATEGORY, first["usage"]
  end

  # @behavior RD-064
  def test_each_class_a_script_inherits_from_is_headed_by_a_category_of_its_own
    names = inherited.map { |member| member["name"] }

    assert_operator names.index("turret.rb"), :<, names.index("mode")
  end

  # @behavior RD-065
  def test_a_nodes_properties_are_headed_by_one_category_for_its_class
    cannon = Loader::Cannon.new

    headings = cannon.get_property_list.select { |member| member["name"] == "cannon.rb" }

    assert_equal 1, headings.size
    assert_equal CATEGORY, headings.first["usage"]
  ensure
    cannon&.free
  end

  private

  # What the cannon's script says it has, its class's and the class it
  # extends alike.
  def inherited
    Loader::Cannon
    Godot::ResourceLoader.load("res://loader/cannon.rb").get_script_property_list
  end


  # What the panel's script says it has, headings and properties alike.
  def listed
    Loader::Panel
    Godot::ResourceLoader.load("res://loader/panel.rb").get_script_property_list
  end
end
