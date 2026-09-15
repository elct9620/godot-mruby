# A test that passes and one that skips: the run has to pass, and leaving the
# skipped test out has to leave no skips.
class SkipTest < Minitest::Test
  def test_passes
    pass
  end

  def test_skips
    skip "skipping is what this test shows"
  end
end
