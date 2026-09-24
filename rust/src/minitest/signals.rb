# The signals a test watches: watch_signals connects to every signal of an
# object and counts its emissions, until the test's teardown disconnects them.
module Minitest
  module SignalWatcher
    def before_setup
      super
      @emissions = {}
      @watching = []
    end

    def watch_signals(object)
      counts = @emissions[object] = {}
      object.get_signal_list.each do |declared|
        name = declared["name"]
        counts[name] = 0
        signal = Godot::Signal.new(object, name)
        # connect_block and disconnect_block come with Waits, which every
        # test class includes too.
        @watching << [signal, connect_block(signal) { counts[name] += 1 }]
      end
      object
    end

    def assert_signal_emitted(object, signal_name = nil, msg = nil)
      object, signal_name, msg = object.get_object, object.get_name, signal_name if object.is_a?(Godot::Signal)
      failure = unemitted(object, signal_name.to_s)
      assert failure.nil?, message(msg) { failure }
    end

    def before_teardown
      @watching.each { |signal, connection| disconnect_block(signal, connection) }
      super
    end

    private

    # Why `name` does not count as emitted by `object`, or nil when it does.
    def unemitted(object, name)
      counts = @emissions[object]
      if counts.nil?
        "Expected #{mu_pp object} to be watched with watch_signals"
      elsif !counts.key?(name)
        "Expected #{mu_pp object} to have a signal #{name}"
      elsif counts[name] == 0
        emitted = counts.keys.select { |other| counts[other] > 0 }
        "Expected #{mu_pp object} to have emitted #{name}, but it emitted #{mu_pp emitted}"
      end
    end
  end

  class Test
    include SignalWatcher
  end
end
