class ObjectScriptsTest < Minitest::Test
  # @behavior RS-086
  def test_an_object_scripts_class_makes_an_object_carrying_its_script
    tally = Unit::Script::Tally.new

    assert_equal 1, tally.call(:bump)
  end

  # @behavior RS-087
  def test_an_object_lives_while_ruby_holds_it
    tally = Unit::Script::Tally.new
    tally.bump

    GC.start
    wait_process_frames(1)

    assert_equal 2, tally.bump
  end

  # @behavior RS-088
  def test_an_object_is_freed_once_neither_ruby_nor_the_engine_holds_it
    id = new_tally_id

    freed = wait_until(1.0) do
      GC.start
      !instance_id_valid?(id)
    end

    assert freed
  end

  # @behavior RS-089
  def test_an_object_the_engine_alone_holds_keeps_its_ruby_state
    holder = Godot::Array.new
    put_bumped_tally(holder)

    GC.start
    wait_process_frames(1)

    assert_equal 1, holder[0].count
  end

  # @behavior RS-090
  def test_an_object_scripts_resource_keeps_its_exports_through_a_save_and_a_load
    ledger = Unit::Script::Ledger.new
    ledger.gold = 42
    path = "user://ledger.tres"

    Godot::ResourceSaver.save(ledger, path)
    loaded = Godot::ResourceLoader.load(path, "", Godot::ResourceLoader::CACHE_MODE_IGNORE)

    assert_equal 42, loaded.get(:gold)
  end

  # @behavior RS-091
  def test_an_object_script_extending_object_makes_an_object_ruby_frees
    token = Unit::Script::Token.new

    assert_equal "token", token.call(:label)
    token.free
  end

  private

  # Made in a fiber that ends, whose registers the collector no longer
  # marks, so nothing but the object's own references keeps it.
  def new_tally_id
    Fiber.new do
      tally = Unit::Script::Tally.new
      Godot::Array.new([tally]).clear
      tally.get_instance_id
    end.resume
  end

  # Asked through an expression, which takes no reference to the object.
  def instance_id_valid?(id)
    expression = Godot::Expression.new
    expression.parse("is_instance_id_valid(id)", ["id"])
    expression.execute([id])
  end

  def put_bumped_tally(holder)
    tally = Unit::Script::Tally.new
    tally.bump
    holder.append(tally)
  end
end
