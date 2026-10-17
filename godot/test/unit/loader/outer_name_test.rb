class OuterNameTest < Minitest::Test
  # @behavior RL-002
  def test_a_namespace_finds_its_own_constant_though_the_outer_one_loaded_first
    Unit::Loader::Bolt

    content = Unit::Loader::Bin::Tray::CONTENT

    assert_same Unit::Loader::Bin::Bolt, content
  end

  # @behavior RL-033
  def test_a_namespace_finds_its_own_constant_though_the_outer_one_loads_later
    Unit::Loader::Rack::Stand
    Unit::Loader::Nut

    content = Unit::Loader::Rack::Shelf::CONTENT

    assert_same Unit::Loader::Rack::Nut, content
  end

  # @behavior RL-034
  def test_a_namespace_loads_the_outer_constant_though_its_own_file_raises
    Unit::Loader::Bay::Dock

    latch = Unit::Loader::Latch

    assert_kind_of Class, latch
    refute Unit::Loader::Bay.const_defined?(:Latch, false)
  end
end
