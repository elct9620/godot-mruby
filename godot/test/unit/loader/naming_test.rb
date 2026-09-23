class NamingTest < Minitest::Test
  # @behavior RL-014
  def test_a_constant_two_files_spell_does_not_load_by_name
    assert_raises(NameError) { Unit::Loader::Naming::HttpClient }
  end
end
