using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using FlowReveal.Core.Models;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Platforms.Windows.Network
{
    public class NetworkAdapterManager
    {
        private readonly ILogger<NetworkAdapterManager> _logger;
        private List<NetworkAdapter> _adapters = new();

        public event EventHandler<IReadOnlyList<NetworkAdapter>>? AdaptersChanged;

        public IReadOnlyList<NetworkAdapter> AvailableAdapters => _adapters.AsReadOnly();

        public NetworkAdapterManager(ILogger<NetworkAdapterManager> logger)
        {
            _logger = logger;
            NetworkChange.NetworkAddressChanged += OnNetworkAddressChanged;
            NetworkChange.NetworkAvailabilityChanged += OnNetworkAvailabilityChanged;
        }

        public IReadOnlyList<NetworkAdapter> RefreshAdapters()
        {
            _logger.LogInformation("Refreshing network adapters...");

            try
            {
                _adapters.Clear();
                var interfaces = NetworkInterface.GetAllNetworkInterfaces();

                foreach (var ni in interfaces)
                {
                    if (ni.NetworkInterfaceType == NetworkInterfaceType.Loopback)
                    {
                        continue;
                    }

                    if (ni.OperationalStatus != OperationalStatus.Up)
                    {
                        continue;
                    }

                    if (ni.NetworkInterfaceType == NetworkInterfaceType.Unknown)
                    {
                        continue;
                    }

                    var name = ni.Name;
                    var desc = ni.Description ?? "";
                    var combined = (name + " " + desc).ToUpperInvariant();

                    var filterSuffixes = new[]
                    {
                        "-WFP ", "-WFP",
                        "-Npcap ", "-Npcap",
                        "-QoS ", "-QoS",
                        "-VirtualBox NDIS",
                        "-WFP 802.3",
                        "LightWeight Filter",
                        "Packet Driver (NPCAP)",
                        "Packet Scheduler"
                    };

                    var isSubAdapter = false;
                    foreach (var suffix in filterSuffixes)
                    {
                        if (combined.Contains(suffix.ToUpperInvariant()))
                        {
                            isSubAdapter = true;
                            break;
                        }
                    }

                    if (isSubAdapter)
                    {
                        continue;
                    }

                    var adapter = new NetworkAdapter
                    {
                        Index = GetInterfaceIndex(ni),
                        Name = ni.Name,
                        Description = ni.Description,
                        FriendlyName = ni.Name,
                        IsUp = ni.OperationalStatus == OperationalStatus.Up,
                        IsLoopback = ni.NetworkInterfaceType == NetworkInterfaceType.Loopback,
                        Speed = ni.Speed,
                        MacAddress = ni.GetPhysicalAddress().GetAddressBytes()
                    };

                    var ipProperties = ni.GetIPProperties();
                    foreach (var ip in ipProperties.UnicastAddresses)
                    {
                        if (ip.Address.AddressFamily == AddressFamily.InterNetwork)
                        {
                            adapter.IpAddresses.Add(ip.Address);
                        }
                    }

                    _adapters.Add(adapter);

                    _logger.LogInformation(
                        "Found adapter: {Name} | IP: {IPs} | MAC: {MAC} | Speed: {Speed}bps | Status: {Status} | Type: {Type}",
                        adapter.FriendlyName,
                        string.Join(", ", adapter.IpAddresses.Select(ip => ip.ToString())),
                        BitConverter.ToString(adapter.MacAddress).Replace("-", ":"),
                        adapter.Speed,
                        adapter.IsUp ? "Up" : "Down",
                        ni.NetworkInterfaceType);
                }

                _logger.LogInformation("Total available adapters: {Count}", _adapters.Count);
                AdaptersChanged?.Invoke(this, _adapters.AsReadOnly());
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to refresh network adapters");
            }

            return _adapters.AsReadOnly();
        }

        public NetworkAdapter? GetAdapterByIndex(int index)
        {
            return _adapters.FirstOrDefault(a => a.Index == index);
        }

        public NetworkAdapter? GetBestAdapter()
        {
            var candidates = _adapters
                .Where(a => a.IsUp && !a.IsLoopback && a.IpAddresses.Count > 0)
                .ToList();

            if (candidates.Count == 0)
            {
                _logger.LogWarning("No suitable network adapter found");
                return null;
            }

            var physicalAdapters = candidates
                .Where(a => !IsVirtualAdapter(a))
                .ToList();

            var selected = physicalAdapters.Count > 0
                ? physicalAdapters.OrderByDescending(a => a.Speed).First()
                : candidates.OrderByDescending(a => a.Speed).First();

            _logger.LogInformation("Selected best adapter: {Name} (IP: {IPs}, Speed: {Speed}bps, Virtual: {IsVirtual})",
                selected.FriendlyName,
                string.Join(", ", selected.IpAddresses),
                selected.Speed,
                IsVirtualAdapter(selected));

            return selected;
        }

        private bool IsVirtualAdapter(NetworkAdapter adapter)
        {
            var name = adapter.Name ?? "";
            var desc = adapter.Description ?? "";
            var combined = (name + " " + desc).ToUpperInvariant();

            var virtualKeywords = new[]
            {
                "VIRTUAL", "VMWARE", "VIRTUALBOX", "VBOX", "HYPER-V", "VETHERNET",
                "DOCKER", "WSL", "TUNNEL", "VPN", "LOOPBACK", "BLUETOOTH",
                "6TO4", "ISATAP", "TEREDO", "PSEUDO", "VMNET", "BRIDGE",
                "HAMACHI", "ZERO TIER", "ZEROTIER", "WIREGUARD", "OPENVPN",
                "TAP-WINDOWS", "NORDVPN", "SURFSHARK"
            };

            foreach (var keyword in virtualKeywords)
            {
                if (combined.Contains(keyword))
                {
                    _logger.LogDebug("Adapter '{Name}' identified as virtual (matched keyword: {Keyword})", adapter.FriendlyName, keyword);
                    return true;
                }
            }

            if (adapter.MacAddress.Length == 0)
            {
                _logger.LogDebug("Adapter '{Name}' identified as virtual (no MAC address)", adapter.FriendlyName);
                return true;
            }

            return false;
        }

        private void OnNetworkAddressChanged(object? sender, EventArgs e)
        {
            _logger.LogInformation("Network address changed, refreshing adapters");
            RefreshAdapters();
        }

        private void OnNetworkAvailabilityChanged(object? sender, NetworkAvailabilityEventArgs e)
        {
            _logger.LogInformation("Network availability changed: {IsAvailable}", e.IsAvailable);
            RefreshAdapters();
        }

        private int GetInterfaceIndex(NetworkInterface ni)
        {
            try
            {
                var ipProperties = ni.GetIPProperties();
                var ipv4Properties = ipProperties.GetIPv4Properties();
                return ipv4Properties?.Index ?? -1;
            }
            catch
            {
                return -1;
            }
        }
    }
}
