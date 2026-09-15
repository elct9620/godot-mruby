# Each test changes what setup gave it and leaves a mark on its instance;
# whichever runs second must see neither.
class SetupTest < Minitest::Test
  def setup
    @steps = [:setup]
  end

  # @behavior RM-001
  def test_one_starts_from_what_setup_set
    assert_equal [:setup], @steps
    assert_nil @mark
    @steps << :one
    @mark = :one
  end

  # @behavior RM-001
  def test_two_starts_from_what_setup_set
    assert_equal [:setup], @steps
    assert_nil @mark
    @steps << :two
    @mark = :two
  end
end
