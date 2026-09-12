# frozen_string_literal: true

require "beni/tasks"
require "rubocop/rake_task"

require_relative "tasks/support/extension"

# mruby is built once per architecture the extension ships: the host build is
# what cargo links by default, and on macOS the x86_64 build is the other half
# of the universal library. Each name here must match a build in the config,
# since beni checks every named build's archive exists after building.
Beni::Tasks.new do
  build_config "build_config/mruby.rb"

  target :host
  target Extension::CROSS_TRIPLE.to_sym if Extension.macos?
end

RuboCop::RakeTask.new

Dir.glob(File.join(__dir__, "tasks", "*.rake")).each { |f| load f }

task default: %i[rubocop extension:build godot:verify]
