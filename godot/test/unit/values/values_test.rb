class ValuesTest < Minitest::Test
  def setup
    @node = autofree(Godot::Node.new)
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

    error = assert_raises(TypeError) { @node.set_process_priority(:hero) }

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
  def test_an_array_and_a_hash_cross_into_the_engine_as_copies_of_their_elements
    list = [1, "two", [:three], Godot::Vector2.new(1, 2), Godot::Color.new(1, 0, 0)]
    table = { "one" => 1, two: [2.0] }
    @node.set_meta(:list, list)
    @node.set_meta(:table, table)

    list << 6
    table[:three] = 3

    assert_operator @node.get_meta(:list), :==, [1, "two", [:three], Godot::Vector2.new(1, 2), Godot::Color.new(1, 0, 0)]
    assert_operator @node.get_meta(:table), :==, { "one" => 1, two: [2.0] }
  end

  # @behavior RV-006
  def test_a_container_nested_too_deep_crosses_as_nothing
    deep = (1..101).reduce([]) { |inner, _| [inner] }
    looped = []
    looped << looped

    assert_raises(ArgumentError) { @node.set_meta(:deep, deep) }
    assert_raises(ArgumentError) { @node.set_meta(:looped, looped) }
    refute @node.has_meta(:deep)
    refute @node.has_meta(:looped)
  end

  # @behavior RV-007
  def test_an_engine_object_crosses_as_the_same_object
    child = Godot::Node.new
    timer = Godot::Timer.new
    beacon = Unit::Script::Beacon.new(1)
    @node.add_child(child)
    @node.add_child(timer)
    @node.add_child(beacon)

    assert_equal child, @node.get_child(0)
    assert_instance_of Godot::Node, @node.get_child(0)
    assert_instance_of Godot::Timer, @node.get_child(1)
    assert_same beacon, @node.get_child(2)
  end

  # @behavior RV-021
  def test_an_engine_object_of_a_hidden_class_crosses_as_its_nearest_exposed_class
    waiter = coroutine_script.new

    suspended = waiter.wait

    assert_instance_of Godot::RefCounted, suspended
    assert suspended.is_valid
    refute_respond_to Godot::RefCounted.new, :is_valid
  end

  # @behavior RV-009
  def test_an_engine_container_reaches_ruby_however_deep_it_nests
    keeper = autofree(Unit::Values::Keeper.new)
    looped = Godot::Array.new
    looped << looped

    keeper.call(:keep, looped)

    assert Godot.is_same(looped, keeper.kept)
  end

  # @behavior RV-009
  def test_an_engine_answer_reaches_ruby_however_deep_it_nests
    deep = ("[" * 101) + ("]" * 101)

    answer = Godot::JSON.parse_string(deep)

    100.times { answer = answer[0] }
    assert_operator answer, :==, []
  end

  # @behavior RV-037
  def test_an_engine_container_reaches_ruby_as_the_one_the_engine_holds
    @node.set_meta(:list, [1])
    @node.set_meta(:table, { "one" => 1 })

    @node.get_meta(:list) << 2
    @node.get_meta(:table)[:two] = 2

    assert_operator @node.get_meta(:list), :==, [1, 2]
    assert_operator @node.get_meta(:table), :==, { "one" => 1, "two" => 2 }
  end

  # @behavior RV-038
  def test_a_containers_copy_is_a_container_of_its_own
    list = Godot::Array.new([1])
    table = Godot::Dictionary.new({ "one" => 1 })

    list.dup << 2
    table.dup["two"] = 2

    assert_operator list, :==, [1]
    assert_operator table, :==, { "one" => 1 }
  end

  # @behavior RV-039
  def test_an_engine_container_equals_rubys_container_of_equal_elements
    list = Godot::Array.new([1, [2], { "three" => 3 }])
    table = Godot::Dictionary.new({ "name" => :hero, level: [1] })

    assert_operator list, :==, [1, [2], { three: 3 }]
    assert_equal 0, list <=> [1, [2], { "three" => 3 }]
    assert_operator table, :==, { name: :hero, "level" => [1] }
  end

  # @behavior RV-042
  def test_rubys_container_equals_an_engine_container_of_equal_elements
    list = Godot::Array.new([1, [2]])
    table = Godot::Dictionary.new({ "name" => :hero })

    assert_operator [1, [2]], :==, list
    assert_operator({ "name" => :hero }, :==, table)
    refute_operator [1], :==, list
  end

  # @behavior RV-040
  def test_a_container_answers_rubys_names_as_ruby_and_the_engines_others_as_the_engine
    list = Godot::Array.new([1, 2])

    doubled = list.map { |element| element * 2 }
    list.push_back(3)

    assert_equal [2, 4], doubled
    assert_operator list, :==, [1, 2, 3]
    assert_equal [2, 3], list[1..2]
  end

  # @behavior RV-041
  def test_a_packed_array_reaches_ruby_as_the_one_the_engine_passes
    holder = packed_holder_script.new

    holder.packed_answer << 2
    holder.get(:packed) << 3
    holder.call(:packed_answer) << 4

    assert_operator holder.get(:packed), :==, [1, 2, 3, 4]
  end

  # @behavior RV-038
  def test_a_packed_arrays_copy_is_one_of_its_own
    packed = Godot::PackedInt32Array.new([1])

    packed.dup << 2

    assert_operator packed, :==, [1]
  end

  private

  # A GDScript whose `wait` suspends, so calling it without `await` answers
  # the GDScriptFunctionState the engine keeps hidden from extensions.
  def coroutine_script
    script = Godot::GDScript.new
    script.source_code = "extends RefCounted\nsignal done\nfunc wait():\n\tawait done\n"
    script.reload
    script
  end

  # What the engine hands back for `value`, given as the default of a
  # metadata entry the node does not have.
  def round_trip(value)
    @node.get_meta(:absent, value)
  end

  # A GDScript holding a packed array, which its method answers.
  def packed_holder_script
    script = Godot::GDScript.new
    script.source_code = "extends RefCounted\nvar packed := PackedInt32Array([1])\nfunc packed_answer():\n\treturn packed\n"
    script.reload
    script
  end
end
