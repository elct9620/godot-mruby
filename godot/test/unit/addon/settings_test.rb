class SettingsTest < Minitest::Test
  # What Godot marks a setting the Project Settings dialog shows without its
  # Advanced Settings toggle with: PROPERTY_USAGE_EDITOR_BASIC_SETTING.
  BASIC = 1 << 27

  # @behavior RA-002
  def test_the_extensions_settings_are_shown_without_advanced_settings
    settings = Godot::ProjectSettings.get_property_list.select { |setting| setting["name"].to_s[0, 6] == "mruby/" }

    refute_empty settings
    settings.each do |setting|
      assert_equal BASIC, setting["usage"] & BASIC, "#{setting["name"]} is an advanced setting"
    end
  end
end
