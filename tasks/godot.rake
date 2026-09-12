# frozen_string_literal: true

require_relative "support/godot"

namespace :godot do
  desc "Load the addon in a headless editor and fail unless the extension initialized"
  task :verify do
    Godot.verify_loaded!
  end
end
