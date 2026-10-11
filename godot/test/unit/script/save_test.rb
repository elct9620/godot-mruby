class SaveTest < Minitest::Test
  SAVE_PATH = "user://save_test.rb".freeze

  def teardown
    Godot::DirAccess.remove_absolute(SAVE_PATH)
  end

  # @behavior RS-049
  def test_godot_saving_a_ruby_script_writes_its_source_to_the_file
    script = Godot::ResourceLoader.load("res://test/unit/script/echo.rb").duplicate
    script.source_code = "# saved\n"

    Godot::ResourceSaver.save(script, SAVE_PATH)

    assert_equal "# saved\n", Godot::FileAccess.get_file_as_string(SAVE_PATH)
  end
end
