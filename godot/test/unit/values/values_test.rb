class ValuesTest < Minitest::Test
  def setup
    @node = Godot::Node.new
  end

  def teardown
    @node.free
  end

  # @behavior RV-001
  def test_a_scalar_crosses_as_itself
    assert_nil round_trip(nil)
    [true, false, 42, 1.5].each do |value|
      assert_equal value, round_trip(value)
      assert_equal value.class, round_trip(value).class
    end
  end

  # @behavior RV-002
  def test_a_string_crosses_as_a_copy
    text = "hero".dup
    @node.set_meta(:text, text)

    text.replace("heroine")

    assert_equal "hero", @node.get_meta(:text)
  end

  # @behavior RV-003
  def test_a_symbol_crosses_as_a_string_name
    @node.set_meta(:name, :hero)

    error = assert_raises(Godot::CallError) { @node.set_process_priority(:hero) }

    assert_equal :hero, @node.get_meta(:name)
    assert_includes error.message, "from StringName to int"
    assert_equal :hero, @node.get_meta(:name)
  end

  # @behavior RV-004
  def test_a_string_is_taken_where_the_engine_wants_a_name_or_a_path
    @node.name = "Hero"
    child = Godot::Node.new
    child.name = "Sword"
    @node.add_child(child)

    assert_equal :Hero, @node.name
    assert_equal child, @node.get_node("Sword")
  end

  # @behavior RV-005
  def test_an_array_and_a_hash_cross_as_copies_of_their_elements
    list = [1, "two", [:three]]
    table = { "one" => 1, two: [2.0] }

    assert_equal list, round_trip(list)
    refute_same list, round_trip(list)
    assert_equal table, round_trip(table)
  end

  # @behavior RV-006
  def test_a_container_nested_too_deep_crosses_as_nothing
    deep = (1..101).reduce([]) { |inner, _| [inner] }
    looped = []
    looped << looped

    assert_raises(Godot::CallError) { @node.set_meta(:deep, deep) }
    assert_raises(Godot::CallError) { @node.set_meta(:looped, looped) }
    refute @node.has_meta(:deep)
  end

  # @behavior RV-007
  def test_an_engine_object_crosses_as_the_same_object
    child = Godot::Node.new
    beacon = Unit::Script::Beacon.new(1)
    @node.add_child(child)
    @node.add_child(beacon)

    assert_equal child, @node.get_child(0)
    assert_same beacon, @node.get_child(1)
  end

  private

  # What the engine hands back for `value`, given as the default of a
  # metadata entry the node does not have.
  def round_trip(value)
    @node.get_meta(:absent, value)
  end
end
