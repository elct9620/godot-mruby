class ClassCallsTest < Minitest::Test
  FORGE = "res://test/unit/script/forge.rb".freeze
  # The engine's MethodFlags.METHOD_FLAG_STATIC.
  STATIC = 32

  # @behavior RS-106
  def test_gdscript_makes_an_object_of_a_ruby_class_with_new
    made = run_gdscript("return load(\"#{FORGE}\").new(5)")

    assert_kind_of Unit::Script::Forge, made
    assert_equal 5, made.heat
  end

  # @behavior RS-107
  def test_gdscript_calls_a_ruby_classs_method_on_self
    assert_equal 6, run_gdscript("return load(\"#{FORGE}\").temper(3)")
  end

  # @behavior RS-108
  def test_a_script_lists_its_classs_methods_on_self_as_static
    script = Godot::ResourceLoader.load(FORGE)
    listed = script.get_script_method_list.find { |method| method["name"] == "temper" }

    assert listed["flags"] & STATIC != 0
    assert_equal 8, run_gdscript("return preload(\"#{FORGE}\").temper(4)")
  end

  # @behavior RS-109
  def test_an_object_answers_its_classs_method_on_self
    forge = Unit::Script::Forge.new

    assert_equal 10, forge.call(:temper, 5)
  end

  private

  # What a GDScript function whose body is `body` answers.
  def run_gdscript(body)
    script = Godot::GDScript.new
    script.source_code = "extends RefCounted\nfunc run():\n\t#{body}\n"
    script.reload
    script.new.call(:run)
  end
end
