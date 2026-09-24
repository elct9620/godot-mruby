class OuterNameTest < Minitest::Test
  # @behavior RL-002
  def test_finds_the_namespaces_own_constant_though_the_outer_one_loaded_first
    Unit::Loader::Bolt

    held = Unit::Loader::Bin::Tray::HOLDS

    assert_same Unit::Loader::Bin::Bolt, held
  end

  # @behavior RL-033
  def test_finds_the_namespaces_own_constant_though_the_outer_one_loads_later
    Unit::Loader::Rack::Stand
    Unit::Loader::Nut

    held = Unit::Loader::Rack::Shelf::HOLDS

    assert_same Unit::Loader::Rack::Nut, held
  end

  # @behavior RL-034
  def test_loads_the_outer_constant_though_the_namespaces_own_file_raises
    Unit::Loader::Bay::Dock

    latch = Unit::Loader::Latch

    assert_kind_of Class, latch
    refute Unit::Loader::Bay.const_defined?(:Latch, false)
  end
end
