# A skipped test ends without failing, so the run holding it still passes.
class SkipTest < Minitest::Test
  def test_a_skipped_test_does_not_fail_the_run
    skip "skipping is what this test shows"
  end
end
