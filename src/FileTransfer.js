import React from 'react';
import './App.css';
import { createStyles, Table, Progress, Anchor, Text, Group, ScrollArea, rem, Box, ActionIcon, white, Badge } from '@mantine/core';
import { IconPlayerPause, IconCheckbox, IconFolder, IconSquareArrowLeft, IconSquareArrowRight, IconTrash, IconPlayerPlay } from '@tabler/icons-react';
import FileAdd from './FileAdd';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { useState, useEffect, useContext } from 'react';
import FileTransferComponent from './FileTransferComponent';
import { MyContext } from './Context';
import { event } from '@tauri-apps/api';



export default function FileTransfer() {
  const { transfers, setTransfer, downloadInfos, setDownloadInfos } = useContext(MyContext);


  useEffect(() => {
    if (downloadInfos.length > 0) {
      downloadInfos.forEach((elem) => {
        const file_hash = elem.file_hash;
        setTransfer((prev) => {
          const temp = prev.filter(e => e.fileHash !== file_hash)
          const matched = prev.find(e => e.fileHash === file_hash)
          const percent = (elem.count/matched.chunks) * 100
          matched.status = percent.toFixed(2);
          matched.down = elem.download_speed
          return [matched, ...temp]
        })
      });
    }

  }, [downloadInfos])
  const rows = transfers.map((row) => {

    return (

      <FileTransferComponent
        key={row.fileHash}
        row={row}
        setTransfer={setTransfer}
      />

    );

  });

  return (
    <>
      <Badge className='font-badge' style={{ fontSize: '25px', padding: '20px', marginTop: '20px' }} color="gray" variant="outline">File Transfer</Badge>

      <ScrollArea sx={{ marginTop: "20px" }}>
        <Table sx={{ minWidth: 800 }} verticalSpacing="sm">
          <thead>
            <tr className='font-header'>
              <th>File</th>
              <th>Down</th>
              <th>Size</th>
              <th>Status</th>
              <th>Remaining</th>
              <th>Seed</th>
            </tr>
          </thead>
          <tbody>{rows}</tbody>
        </Table>
      </ScrollArea>

      <FileAdd />
    </>
  );

}