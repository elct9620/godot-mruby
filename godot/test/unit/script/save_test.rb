class SaveTest < Minitest::Test
  SAVED = "user://save_test.rb".freeze

  def teardown
    Godot::DirAccess.remove_absolute(SAVED)
  end

  # @behavior RS-049
  def test_godot_saving_a_ruby_script_writes_its_source_to_the_file
    script = Godot::ResourceLoader.load("res://test/unit/script/echo.rb").duplicate
    script.source_code = "# saved\n"

    Godot::ResourceSaver.save(script, SAVED)

    assert_equal "# saved\n", Godot::FileAccess.get_file_as_string(SAVED)
  end
end
