class PrintTest < Minitest::Test
  # @behavior RS-027
  def test_p_answers_what_it_is_given
    assert_nil p
    assert_equal :one, p(:one)
    assert_equal %i[one two], p(:one, :two)
  end
end
