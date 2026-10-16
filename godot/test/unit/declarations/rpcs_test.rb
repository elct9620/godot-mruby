class RpcsTest < Minitest::Test
  # @behavior RD-114
  def test_a_scripts_rpc_config_names_its_rpcs_as_gdscripts_does
    Unit::Declarations::Courier
    config = Godot::ResourceLoader.load("res://test/unit/declarations/courier.rb").get_rpc_config

    assert_operator config[:deliver], :==, { "rpc_mode" => 2 }
    assert_operator config[:announce], :==, { "rpc_mode" => 1, "call_local" => true, "transfer_mode" => 1, "channel" => 2 }
  end

  # @behavior RD-115
  def test_a_node_calling_an_rpc_that_runs_locally_runs_it
    courier = add_child_autofree(Unit::Declarations::Courier.new)

    courier.rpc(:announce)

    assert_equal 1, courier.announced
  end

  # @behavior RD-116
  def test_an_rpc_with_a_word_rpc_does_not_take_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Stray }

    assert_includes error.message, %(Invalid RPC argument "everyone".)
  end
end
